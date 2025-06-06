//! Structured logging using JSON (NDJSON).

use std::fmt::{self, Write};
use std::io::IoSlice;

use log::kv::{VisitSource, VisitValue};
use log::{kv, Record};

#[cfg(feature = "timestamp")]
use crate::format::format_timestamp;
use crate::format::{Buffer, Format, BUFS_SIZE};

/// Structured logging using JSON.
#[allow(missing_debug_implementations)]
pub enum Json {}

impl Format for Json {
    fn format<'b, Kvs: kv::Source>(
        bufs: &'b mut [IoSlice<'b>; BUFS_SIZE],
        buf: &'b mut Buffer,
        record: &'b Record,
        kvs: &Kvs,
        add_loc: bool,
    ) -> &'b [IoSlice<'b>] {
        // Write all parts of the buffer that need formatting.
        buf.buf[0] = b'{';
        #[cfg(feature = "timestamp")]
        write_timestamp(buf);
        write_msg(buf, record.args());
        write_key_values(buf, record.key_values(), kvs);
        if add_loc {
            write_line(buf, record.line().unwrap_or(0));
        }

        // Now that we've written the message to our buffer we have to construct
        // it. The first part of the message is the timestamp and log level,
        // e.g. `{"timestamp":"2020-12-31T12:32:23.906132Z","level":"INFO`.
        // Or without a timestamp, i.e. `{"level":"INFO`.
        bufs[0] = IoSlice::new(timestamp(buf));
        bufs[1] = IoSlice::new(b"\"level\":\"");
        bufs[2] = IoSlice::new(record.level().as_str().as_bytes());
        // The message (and the end of the log level), e.g. `","message":"some message`.
        bufs[3] = IoSlice::new(b"\",\"message\":\"");
        bufs[4] = IoSlice::new(msg(buf));
        // The target, e.g. `","target":"request`.
        bufs[5] = IoSlice::new(b"\",\"target\":\"");
        bufs[6] = IoSlice::new(record.target().as_bytes());
        // The module, e.g. `","module":"stored::http`.
        bufs[7] = IoSlice::new(b"\",\"module\":\"");
        bufs[8] = IoSlice::new(record.module_path().unwrap_or("").as_bytes());
        // Any key value pairs supplied by the user.
        bufs[9] = IoSlice::new(key_values(buf));
        // Optional file, e.g. `","file":"some_file.rs","line":"123"}`, and a
        // line end.
        let n = if add_loc {
            bufs[10] = IoSlice::new(b",\"file\":\"");
            bufs[11] = IoSlice::new(record.file().unwrap_or("??").as_bytes());
            bufs[12] = IoSlice::new(b"\",\"line\":\"");
            bufs[13] = IoSlice::new(line(buf));
            bufs[14] = IoSlice::new(b"\"}\n");
            15
        } else {
            bufs[10] = IoSlice::new(b"}\n");
            11
        };
        &bufs[..n]
    }
}

/// Index of the end of `{"timestamp":"0000-00-00T00:00:00.000000Z",`.
#[cfg(feature = "timestamp")]
const TS_END_INDEX: usize = 43;
#[cfg(not(feature = "timestamp"))]
const TS_END_INDEX: usize = 1;

#[inline]
#[cfg(feature = "timestamp")]
pub(crate) fn write_timestamp(buf: &mut Buffer) {
    let _ = buf.buf[TS_END_INDEX];
    buf.buf[1] = b'"';
    buf.buf[2] = b't';
    buf.buf[3] = b'i';
    buf.buf[4] = b'm';
    buf.buf[5] = b'e';
    buf.buf[6] = b's';
    buf.buf[7] = b't';
    buf.buf[8] = b'a';
    buf.buf[9] = b'm';
    buf.buf[10] = b'p';
    buf.buf[11] = b'"';
    buf.buf[12] = b':';
    buf.buf[13] = b'"';
    format_timestamp(&mut buf.buf[14..]);
    buf.buf[TS_END_INDEX - 2] = b'"';
    buf.buf[TS_END_INDEX - 1] = b',';
}

#[inline]
pub(crate) fn timestamp(buf: &Buffer) -> &[u8] {
    &buf.buf[..TS_END_INDEX]
}

#[inline]
pub(crate) fn write_msg(buf: &mut Buffer, args: &fmt::Arguments) {
    buf.buf.truncate(TS_END_INDEX);
    if let Some(msg) = args.as_str() {
        Buf(&mut buf.buf)
            .write_str(msg)
            .unwrap_or_else(|_| unreachable!());
    } else {
        Buf(&mut buf.buf)
            .write_fmt(*args)
            .unwrap_or_else(|_| unreachable!());
    }
    buf.indices[0] = buf.buf.len();
}

#[inline]
pub(crate) fn msg(buf: &Buffer) -> &[u8] {
    &buf.buf[TS_END_INDEX..buf.indices[0]]
}

#[inline]
pub(crate) fn write_key_values<Kvs: kv::Source>(
    buf: &mut Buffer,
    kvs1: &dyn kv::Source,
    kvs2: Kvs,
) {
    buf.buf.extend_from_slice(b"\"");
    // TODO: see if we can add to the slice of `IoSlice` using the keys
    // and string values.
    let mut visitor = KeyValueVisitor(&mut buf.buf);
    kvs1.visit(&mut visitor).unwrap_or_else(|_| unreachable!());
    kvs2.visit(&mut visitor).unwrap_or_else(|_| unreachable!());
    buf.indices[1] = buf.buf.len();
}

#[inline]
pub(crate) fn key_values(buf: &Buffer) -> &[u8] {
    &buf.buf[buf.indices[0]..buf.indices[1]]
}

#[inline]
pub(crate) fn write_line(buf: &mut Buffer, line: u32) {
    let mut itoa = itoa::Buffer::new();
    buf.buf.extend_from_slice(itoa.format(line).as_bytes());
    buf.indices[2] = buf.buf.len();
}

#[inline]
pub(crate) fn line(buf: &Buffer) -> &[u8] {
    &buf.buf[buf.indices[1]..buf.indices[2]]
}

/// Formats key value pairs as a part of an JSON object, in the following
/// format: `"key":"value"`. For example:
/// `"user_name":"Thomas","user_id":123,"is_admin":true`.
pub(super) struct KeyValueVisitor<'b>(pub(super) &'b mut Vec<u8>);

impl<'b, 'kvs> VisitSource<'kvs> for KeyValueVisitor<'b> {
    fn visit_pair(&mut self, key: kv::Key<'kvs>, value: kv::Value<'kvs>) -> Result<(), kv::Error> {
        self.0.push(b',');
        self.0.push(b'"');
        let _ = fmt::Write::write_str(&mut Buf(self.0), key.as_str());
        self.0.push(b'"');
        self.0.push(b':');
        #[cfg(feature = "serde1")]
        serde::Serialize::serialize(&value, self).map_err(kv::Error::boxed)?;
        #[cfg(not(feature = "serde1"))]
        value.visit(self)?;
        Ok(())
    }
}

impl<'b, 'v> VisitValue<'v> for KeyValueVisitor<'b> {
    fn visit_any(&mut self, value: kv::Value) -> Result<(), kv::Error> {
        self.0.push(b'\"');
        Buf(self.0)
            .write_fmt(format_args!("{value}"))
            .unwrap_or_else(|_| unreachable!());
        self.0.push(b'\"');
        Ok(())
    }

    fn visit_null(&mut self) -> Result<(), kv::Error> {
        self.0.extend_from_slice(b"null");
        Ok(())
    }

    fn visit_u64(&mut self, value: u64) -> Result<(), kv::Error> {
        let mut itoa = itoa::Buffer::new();
        self.0.extend_from_slice(itoa.format(value).as_bytes());
        Ok(())
    }

    fn visit_i64(&mut self, value: i64) -> Result<(), kv::Error> {
        let mut itoa = itoa::Buffer::new();
        self.0.extend_from_slice(itoa.format(value).as_bytes());
        Ok(())
    }

    fn visit_u128(&mut self, value: u128) -> Result<(), kv::Error> {
        let mut itoa = itoa::Buffer::new();
        self.0.extend_from_slice(itoa.format(value).as_bytes());
        Ok(())
    }

    fn visit_i128(&mut self, value: i128) -> Result<(), kv::Error> {
        let mut itoa = itoa::Buffer::new();
        self.0.extend_from_slice(itoa.format(value).as_bytes());
        Ok(())
    }

    fn visit_f64(&mut self, value: f64) -> Result<(), kv::Error> {
        let mut ryu = ryu::Buffer::new();
        self.0.extend_from_slice(ryu.format(value).as_bytes());
        Ok(())
    }

    fn visit_bool(&mut self, value: bool) -> Result<(), kv::Error> {
        self.0
            .extend_from_slice(if value { b"true" } else { b"false" });
        Ok(())
    }

    fn visit_str(&mut self, value: &str) -> Result<(), kv::Error> {
        self.0.push(b'\"');
        let _ = fmt::Write::write_str(&mut Buf(self.0), value);
        self.0.push(b'\"');
        Ok(())
    }
}

#[cfg(feature = "serde1")]
impl<'b> serde::Serializer for &mut KeyValueVisitor<'b> {
    type Ok = ();
    type Error = std::fmt::Error; // Unused.
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_bool(v);
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_i64(v as i64);
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_i64(v as i64);
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_i64(v as i64);
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_i64(v);
        Ok(())
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_i128(v);
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_u64(v as u64);
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_u64(v as u64);
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_u64(v as u64);
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_u64(v);
        Ok(())
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_u128(v);
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_f64(v as f64);
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_f64(v);
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        // A char encoded as UTF-8 takes 4 bytes at most.
        let mut buf = [0; 4];
        self.serialize_str(v.encode_utf8(&mut buf))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_str(v);
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        use serde::ser::SerializeSeq;
        // TODO: consider base64 encoding or something.
        let mut serializer = self.serialize_seq(Some(v.len()))?;
        for b in v {
            serializer.serialize_element(b)?;
        }
        serializer.end()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        let _ = self.visit_null();
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        // Serialise as a map using the variant name as key and the value as value.
        let mut serializer = self.serialize_struct(name, 1)?;
        serde::ser::SerializeStruct::serialize_field(&mut serializer, variant, value)?;
        serde::ser::SerializeStruct::end(serializer)
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.0.push(b'[');
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        // Serialise as a map.
        let mut serializer = self.serialize_struct(name, 1)?;
        serde::ser::SerializeMap::serialize_key(&mut serializer, variant)?;
        serializer.serialize_seq(Some(len))
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.0.push(b'{');
        Ok(self)
    }

    fn serialize_struct(
        self,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn collect_str<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + std::fmt::Display,
    {
        self.0.push(b'\"');
        Buf(self.0)
            .write_fmt(format_args!("{value}"))
            .unwrap_or_else(|_| unreachable!());
        self.0.push(b'\"');
        Ok(())
    }

    fn is_human_readable(&self) -> bool {
        true
    }
}

#[cfg(feature = "serde1")]
impl<'b> serde::ser::SerializeSeq for &mut KeyValueVisitor<'b> {
    type Ok = ();
    type Error = std::fmt::Error; // Unused.

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(&mut **self)?;
        self.0.push(b',');
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let _ = self.0.pop_if(|b| *b == b',');
        self.0.push(b']');
        Ok(())
    }
}

#[cfg(feature = "serde1")]
impl<'b> serde::ser::SerializeTuple for &mut KeyValueVisitor<'b> {
    type Ok = ();
    type Error = std::fmt::Error; // Unused.

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

#[cfg(feature = "serde1")]
impl<'b> serde::ser::SerializeTupleStruct for &mut KeyValueVisitor<'b> {
    type Ok = ();
    type Error = std::fmt::Error; // Unused.

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

#[cfg(feature = "serde1")]
impl<'b> serde::ser::SerializeTupleVariant for &mut KeyValueVisitor<'b> {
    type Ok = ();
    type Error = std::fmt::Error; // Unused.

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

#[cfg(feature = "serde1")]
impl<'b> serde::ser::SerializeMap for &mut KeyValueVisitor<'b> {
    type Ok = ();
    type Error = std::fmt::Error; // Unused.

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        key.serialize(&mut **self)?;
        self.0.push(b':');
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(&mut **self)?;
        self.0.push(b',');
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let _ = self.0.pop_if(|b| *b == b',');
        self.0.push(b'}');
        Ok(())
    }
}

#[cfg(feature = "serde1")]
impl<'b> serde::ser::SerializeStruct for &mut KeyValueVisitor<'b> {
    type Ok = ();
    type Error = std::fmt::Error; // Unused.

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        serde::ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        serde::ser::SerializeMap::end(self)
    }
}

#[cfg(feature = "serde1")]
impl<'b> serde::ser::SerializeStructVariant for &mut KeyValueVisitor<'b> {
    type Ok = ();
    type Error = std::fmt::Error; // Unused.

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        serde::ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        serde::ser::SerializeMap::end(self)
    }
}

/// [`fmt::Write`] implementation that writes escaped JSON strings.
pub(super) struct Buf<'b>(pub(super) &'b mut Vec<u8>);

impl<'b> fmt::Write for Buf<'b> {
    #[inline]
    fn write_str(&mut self, string: &str) -> fmt::Result {
        for c in string.chars() {
            let _ = self.write_char(c);
        }
        Ok(())
    }

    #[inline]
    fn write_char(&mut self, c: char) -> fmt::Result {
        // See RFC 8259, section 7
        // <https://datatracker.ietf.org/doc/html/rfc8259#section-7>.
        let mut bytes = [0; 8];
        let bytes: &[u8] = match c {
            // Quotation mark.
            '"' => b"\\\"",
            // Reverse solidus.
            '\\' => b"\\\\",
            // Backspace.
            '\u{0008}' => b"\\b",
            // Form feed.
            '\u{000C}' => b"\\f",
            // Line feed.
            '\u{000A}' => b"\\n",
            // Carriage return.
            '\u{000D}' => b"\\r",
            // Tab.
            '\u{0009}' => b"\\t",
            // Control characters (U+0000 through U+001F).
            '\u{0000}'..='\u{001F}' => {
                bytes[0] = b'\\';
                bytes[1] = b'u';
                bytes[2] = b'0';
                bytes[3] = b'0';
                let [b1, b2] = hex(c as u8);
                bytes[4] = b1;
                bytes[5] = b2;
                &bytes
            }
            _ => c.encode_utf8(&mut bytes).as_bytes(),
        };
        self.0.extend_from_slice(bytes);
        Ok(())
    }
}

#[inline]
const fn hex(c: u8) -> [u8; 2] {
    const HEX: [u8; 16] = *b"0123456789abcdef";
    [HEX[(c >> 4) as usize], HEX[(c & 0b1111) as usize]]
}
