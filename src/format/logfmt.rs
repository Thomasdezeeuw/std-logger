//! Logfmt following <https://www.brandur.org/logfmt>.

use std::fmt;
use std::io::IoSlice;
use std::io::Write;

use log::{kv, Record};

#[cfg(feature = "timestamp")]
use crate::format::format_timestamp;
use crate::format::{Buffer, BUFS_SIZE};

/// Parts of the message we can reuse.
#[cfg(feature = "timestamp")]
pub(crate) const REUSABLE_PARTS: &[u8] = b"ts=\"0000-00-00T00:00:00.000000Z\" lvl=\"\" msg=\"";
#[cfg(not(feature = "timestamp"))]
pub(crate) const REUSABLE_PARTS: &[u8] = b"lvl=\"\" msg=\"";

/// Index of the end of `ts="..." lvl="`.
#[cfg(feature = "timestamp")]
const TS_END_INDEX: usize = 38;
#[cfg(not(feature = "timestamp"))]
const TS_END_INDEX: usize = 5;
/// Index where the message should be written to.
const MSG_START_INDEX: usize = TS_END_INDEX + 7;

/// Formats a log `record`.
///
/// This writes into the buffer `buf` for things that need formatting, which it
/// resets itself. The returned slices is based on `bufs`, which is used to
/// order the writable buffers.
///
/// If `debug` is `true` the file and line are added.
#[inline]
pub(crate) fn format<'b>(
    bufs: &'b mut [IoSlice<'b>; BUFS_SIZE],
    buf: &'b mut Buffer,
    record: &'b Record,
    debug: bool,
) -> &'b [IoSlice<'b>] {
    // Write all parts of the buffer that need formatting.
    #[cfg(feature = "timestamp")]
    write_timestamp(buf);
    write_msg(buf, record.args());
    write_key_values(buf, record.key_values());
    if debug {
        write_line(buf, record.line().unwrap_or(0));
    }

    // Now that we've written the message to our buffer we have to construct it.
    // The first part of the message is the timestamp and log level, e.g.
    // `ts="2020-12-31T12:32:23.906132Z" lvl="INFO`.
    // Or without a timestamp, i.e. `lvl="INFO`.
    bufs[0] = IoSlice::new(timestamp(buf));
    bufs[1] = IoSlice::new(record.level().as_str().as_bytes());
    // The message (and the end of the log level), e.g. `" msg="some message`.
    bufs[2] = IoSlice::new(msg(buf));
    // The target, e.g. `" target="request`.
    bufs[3] = IoSlice::new(b"\" target=\"");
    bufs[4] = IoSlice::new(record.target().as_bytes());
    // The module, e.g. `" module="stored::http`.
    bufs[5] = IoSlice::new(b"\" module=\"");
    bufs[6] = IoSlice::new(record.module_path().unwrap_or("").as_bytes());
    // Any key value pairs supplied by the user.
    bufs[7] = IoSlice::new(key_values(buf));
    // Optional file, e.g. ` file="some_file:123"`, and a line end.
    let n = if debug {
        bufs[8] = IoSlice::new(b" file=\"");
        bufs[9] = IoSlice::new(record.file().unwrap_or("??").as_bytes());
        bufs[10] = IoSlice::new(line(buf));
        11
    } else {
        bufs[8] = IoSlice::new(b"\n");
        9
    };

    &bufs[..n]
}

#[inline]
#[cfg(feature = "timestamp")]
fn write_timestamp(buf: &mut Buffer) {
    format_timestamp(&mut buf.buf[4..]);
}

#[inline]
fn timestamp(buf: &Buffer) -> &[u8] {
    &buf.buf[..TS_END_INDEX]
}

#[inline]
fn write_msg(buf: &mut Buffer, args: &fmt::Arguments) {
    buf.buf.truncate(MSG_START_INDEX);
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
    // NOTE: not using `MSG_START_INDEX` here because we need to include the
    // `" msg="` format part.
    &buf.buf[TS_END_INDEX..buf.indices[0]]
}

#[inline]
fn write_key_values(buf: &mut Buffer, kvs: &dyn kv::Source) {
    buf.buf.extend_from_slice(b"\"");
    // TODO: see if we can add to the slice of `IoSlice` using the keys
    // and string values.
    let mut visitor = KeyValueVisitor(&mut buf.buf);
    kvs.visit(&mut visitor).unwrap_or_else(|_| unreachable!());
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
        // TODO: use key-value visitor proposed here:
        // <https://github.com/rust-lang/log/issues/440>.
        if let Some(value) = value.to_borrowed_str() {
            self.0.push(b'\"');
            self.0.extend_from_slice(value.as_bytes());
            self.0.push(b'\"');
        } else if let Some(value) = value.to_u64() {
            let mut itoa = itoa::Buffer::new();
            self.0.extend_from_slice(itoa.format(value).as_bytes());
        } else if let Some(value) = value.to_i64() {
            let mut itoa = itoa::Buffer::new();
            self.0.extend_from_slice(itoa.format(value).as_bytes());
        } else if let Some(value) = value.to_f64() {
            let mut ryu = ryu::Buffer::new();
            self.0.extend_from_slice(ryu.format(value).as_bytes());
        } else if let Some(value) = value.to_bool() {
            self.0
                .extend_from_slice(if value { b"true" } else { b"false" });
        } else if let Some(value) = value.to_char() {
            self.0.push(b'\"');
            let mut buf = [0; 4];
            self.0
                .extend_from_slice(value.encode_utf8(&mut buf).as_bytes());
            self.0.push(b'\"');
        } else {
            self.0.push(b'\"');
            write!(self.0, "{}", value).unwrap_or_else(|_| unreachable!());
            self.0.push(b'\"');
        }
        Ok(())
    }
}
