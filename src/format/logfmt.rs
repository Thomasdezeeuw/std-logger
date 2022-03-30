//! Logfmt following <https://www.brandur.org/logfmt>.

use std::fmt;
use std::io::IoSlice;
use std::io::Write;

use log::kv::value::Visit;
use log::{kv, Record};

#[cfg(feature = "timestamp")]
use crate::format::format_timestamp;
use crate::format::{Buffer, Format, BUFS_SIZE};

/// Logfmt following <https://www.brandur.org/logfmt>.
#[allow(missing_debug_implementations)]
pub enum LogFmt {}

impl Format for LogFmt {
    fn format<'b, Kvs: kv::Source>(
        bufs: &'b mut [IoSlice<'b>; BUFS_SIZE],
        buf: &'b mut Buffer,
        record: &'b Record,
        kvs: &Kvs,
        debug: bool,
    ) -> &'b [IoSlice<'b>] {
        // Write all parts of the buffer that need formatting.
        #[cfg(feature = "timestamp")]
        write_timestamp(buf);
        write_msg(buf, record.args());
        write_key_values(buf, record.key_values(), kvs);
        if debug {
            write_line(buf, record.line().unwrap_or(0));
        }

        // Now that we've written the message to our buffer we have to construct it.
        // The first part of the message is the timestamp and log level, e.g.
        // `ts="2020-12-31T12:32:23.906132Z" lvl="INFO`.
        // Or without a timestamp, i.e. `lvl="INFO`.
        bufs[0] = IoSlice::new(timestamp(buf));
        bufs[1] = IoSlice::new(b"lvl=\"");
        bufs[2] = IoSlice::new(record.level().as_str().as_bytes());
        bufs[3] = IoSlice::new(b"\" msg=\"");
        // The message (and the end of the log level), e.g. `" msg="some message`.
        bufs[4] = IoSlice::new(msg(buf));
        // The target, e.g. `" target="request`.
        bufs[5] = IoSlice::new(b"\" target=\"");
        bufs[6] = IoSlice::new(record.target().as_bytes());
        // The module, e.g. `" module="stored::http`.
        bufs[7] = IoSlice::new(b"\" module=\"");
        bufs[8] = IoSlice::new(record.module_path().unwrap_or("").as_bytes());
        // Any key value pairs supplied by the user.
        bufs[9] = IoSlice::new(key_values(buf));
        // Optional file, e.g. ` file="some_file:123"`, and a line end.
        let n = if debug {
            bufs[10] = IoSlice::new(b" file=\"");
            bufs[11] = IoSlice::new(record.file().unwrap_or("??").as_bytes());
            bufs[12] = IoSlice::new(line(buf));
            13
        } else {
            bufs[10] = IoSlice::new(b"\n");
            11
        };

        &bufs[..n]
    }
}

/// Index of the end of `ts="..."`.
#[cfg(feature = "timestamp")]
const TS_END_INDEX: usize = 33;
#[cfg(not(feature = "timestamp"))]
const TS_END_INDEX: usize = 0;

#[inline]
#[cfg(feature = "timestamp")]
fn write_timestamp(buf: &mut Buffer) {
    let _ = buf.buf[TS_END_INDEX];
    buf.buf[0] = b't';
    buf.buf[1] = b's';
    buf.buf[2] = b'=';
    buf.buf[3] = b'"';
    format_timestamp(&mut buf.buf[4..]);
    buf.buf[TS_END_INDEX - 2] = b'"';
    buf.buf[TS_END_INDEX - 1] = b' ';
}

#[inline]
fn timestamp(buf: &Buffer) -> &[u8] {
    &buf.buf[..TS_END_INDEX]
}

#[inline]
fn write_msg(buf: &mut Buffer, args: &fmt::Arguments) {
    buf.buf.truncate(TS_END_INDEX);
    #[cfg(not(feature = "nightly"))]
    write!(buf.buf, "{}", args).unwrap_or_else(|_| unreachable!());
    #[cfg(feature = "nightly")]
    if let Some(msg) = args.as_str() {
        buf.buf.extend_from_slice(msg.as_bytes());
    } else {
        write!(buf.buf, "{}", args).unwrap_or_else(|_| unreachable!());
    }
    buf.indices[0] = buf.buf.len();
}

#[inline]
fn msg(buf: &Buffer) -> &[u8] {
    &buf.buf[TS_END_INDEX..buf.indices[0]]
}

#[inline]
fn write_key_values<Kvs: kv::Source>(buf: &mut Buffer, kvs1: &dyn kv::Source, kvs2: Kvs) {
    buf.buf.extend_from_slice(b"\"");
    // TODO: see if we can add to the slice of `IoSlice` using the keys
    // and string values.
    let mut visitor = KeyValueVisitor(&mut buf.buf);
    kvs1.visit(&mut visitor).unwrap_or_else(|_| unreachable!());
    kvs2.visit(&mut visitor).unwrap_or_else(|_| unreachable!());
    buf.indices[1] = buf.buf.len();
}

#[inline]
fn key_values(buf: &Buffer) -> &[u8] {
    &buf.buf[buf.indices[0]..buf.indices[1]]
}

#[inline]
fn write_line(buf: &mut Buffer, line: u32) {
    buf.buf.push(b':');
    let mut itoa = itoa::Buffer::new();
    buf.buf.extend_from_slice(itoa.format(line).as_bytes());
    buf.buf.extend_from_slice(b"\"\n");
    buf.indices[2] = buf.buf.len();
}

#[inline]
fn line(buf: &Buffer) -> &[u8] {
    &buf.buf[buf.indices[1]..buf.indices[2]]
}

/// Formats key value pairs in the following format: `key="value"`. For example:
/// `user_name="Thomas" user_id=123 is_admin=true`
struct KeyValueVisitor<'b>(&'b mut Vec<u8>);

impl<'b, 'kvs> kv::Visitor<'kvs> for KeyValueVisitor<'b> {
    fn visit_pair(&mut self, key: kv::Key<'kvs>, value: kv::Value<'kvs>) -> Result<(), kv::Error> {
        self.0.push(b' ');
        self.0.extend_from_slice(key.as_str().as_bytes());
        self.0.push(b'=');
        value.visit(self)
    }
}

impl<'b, 'v> Visit<'v> for KeyValueVisitor<'b> {
    fn visit_any(&mut self, value: kv::Value) -> Result<(), kv::Error> {
        self.0.push(b'\"');
        write!(self.0, "{}", value).unwrap_or_else(|_| unreachable!());
        self.0.push(b'\"');
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
        self.0.extend_from_slice(value.as_bytes());
        self.0.push(b'\"');
        Ok(())
    }
}
