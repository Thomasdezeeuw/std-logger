//! Logfmt following <https://www.brandur.org/logfmt>.

use std::fmt::{self, Write};

use log::kv::{VisitSource, VisitValue};
use log::{kv, Record};

#[cfg(feature = "timestamp")]
use crate::format::format_timestamp;
use crate::format::Format;

/// Logfmt following <https://www.brandur.org/logfmt>.
#[allow(missing_debug_implementations)]
pub enum LogFmt {}

impl Format for LogFmt {
    fn format<'b, Kvs: kv::Source>(
        buf: &'b mut Vec<u8>,
        record: &'b Record,
        kvs: &Kvs,
        add_loc: bool,
    ) {
        buf.truncate(TS_END_INDEX + 5);
        // The first part of the message is the timestamp, if enabled, e.g.
        // `ts="2020-12-31T12:32:23.906132Z"`.
        #[cfg(feature = "timestamp")]
        write_timestamp(buf);

        // Next is the log level, e.g. `lvl="INFO`. The ending qoute is added
        // with the opening of the message below.
        buf[TS_END_INDEX..TS_END_INDEX + 5].copy_from_slice(b"lvl=\"");
        buf.extend_from_slice(record.level().as_str().as_bytes());

        // The message (and the end of the log level), e.g. `" msg="some
        // message`.
        buf.extend_from_slice(b"\" msg=\"");
        write_msg(buf, record.args());
        buf.extend_from_slice(b"\"");

        // Next are the key-value pairs. Each pair start with a space so we
        // don't have to append one after the message (and if there are no pairs
        // we don't have to do anything).
        write_key_values(buf, record.key_values(), kvs);

        // The target, e.g. ` target="request`.
        buf.extend_from_slice(b" target=\"");
        buf.extend_from_slice(record.target().as_bytes());

        // The module, e.g. `" module="stored::http`.
        buf.extend_from_slice(b"\" module=\"");
        buf.extend_from_slice(record.module_path().unwrap_or("").as_bytes());

        // Optional file, e.g. ` file="some_file:123"`, and a line end.
        if add_loc {
            buf.extend_from_slice(b"\" file=\"");
            buf.extend_from_slice(record.file().unwrap_or("??").as_bytes());
            buf.push(b':');
            let mut itoa = itoa::Buffer::new();
            buf.extend_from_slice(itoa.format(record.line().unwrap_or(0)).as_bytes());
        }

        buf.extend_from_slice(b"\"\n");
    }
}

/// Index of the end of `ts="..."`.
#[cfg(feature = "timestamp")]
const TS_END_INDEX: usize = 33;
#[cfg(not(feature = "timestamp"))]
const TS_END_INDEX: usize = 0;

#[inline]
#[cfg(feature = "timestamp")]
fn write_timestamp(buf: &mut Vec<u8>) {
    buf[0..4].copy_from_slice(b"ts=\"");
    format_timestamp(&mut buf[4..]);
    buf[31..33].copy_from_slice(b"\" ");
}

#[inline]
fn write_msg(buf: &mut Vec<u8>, args: &fmt::Arguments) {
    if let Some(msg) = args.as_str() {
        Buf(buf).write_str(msg).unwrap_or_else(|_| unreachable!());
    } else {
        Buf(buf).write_fmt(*args).unwrap_or_else(|_| unreachable!());
    }
}

#[inline]
fn write_key_values<Kvs: kv::Source>(buf: &mut Vec<u8>, kvs1: &dyn kv::Source, kvs2: Kvs) {
    let mut visitor = KeyValueVisitor(buf);
    kvs1.visit(&mut visitor).unwrap_or_else(|_| unreachable!());
    kvs2.visit(&mut visitor).unwrap_or_else(|_| unreachable!());
}

/// Formats key value pairs in the following format: `key="value"`. For example:
/// `user_name="Thomas" user_id=123 is_admin=true`
struct KeyValueVisitor<'b>(&'b mut Vec<u8>);

impl<'b, 'kvs> VisitSource<'kvs> for KeyValueVisitor<'b> {
    fn visit_pair(&mut self, key: kv::Key<'kvs>, value: kv::Value<'kvs>) -> Result<(), kv::Error> {
        self.0.push(b' ');
        Buf(self.0).extend_from_slice(key.as_str().as_bytes());
        self.0.push(b'=');
        value.visit(self)
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
        let mut buf = zmij::Buffer::new();
        self.0.extend_from_slice(buf.format(value).as_bytes());
        Ok(())
    }

    fn visit_bool(&mut self, value: bool) -> Result<(), kv::Error> {
        self.0
            .extend_from_slice(if value { b"true" } else { b"false" });
        Ok(())
    }

    fn visit_str(&mut self, value: &str) -> Result<(), kv::Error> {
        self.0.push(b'\"');
        Buf(self.0)
            .write_str(value)
            .unwrap_or_else(|_| unreachable!());
        self.0.push(b'\"');
        Ok(())
    }
}

/// [`fmt::Write`] implementation that writes escaped quotes.
struct Buf<'b>(&'b mut Vec<u8>);

impl<'b> Buf<'b> {
    #[inline]
    fn extend_from_slice(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if b == b'"' {
                self.0.push(b'\\');
            }
            self.0.push(b);
        }
    }
}

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
            // Line feed.
            '\u{000A}' => b"\\n",
            // Carriage return.
            '\u{000D}' => b"\\r",
            // Tab.
            '\u{0009}' => b"\\t",
            _ => c.encode_utf8(&mut bytes).as_bytes(),
        };
        self.0.extend_from_slice(bytes);
        Ok(())
    }
}
