//! Google Cloud Platform structured logging using JSON, following
//! <https://cloud.google.com/logging/docs/structured-logging>.

use std::fmt::{self, Write};
use std::io::IoSlice;

use log::{kv, Record};

#[cfg(feature = "timestamp")]
use crate::format::format_timestamp;
use crate::format::json;
use crate::format::{Buffer, Format, BUFS_SIZE};
use crate::PANIC_TARGET;

/// Google Cloud Platform structured logging using JSON, following
/// <https://cloud.google.com/logging/docs/structured-logging>.
#[allow(missing_debug_implementations)]
pub enum Gcloud {}

impl Format for Gcloud {
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

        // Now that we've written the message to our buffer we have to construct it.
        // The first part of the message is the timestamp and log level (severity),
        // e.g. `{"timestamp":"2020-12-31T12:32:23.906132Z","severity":"INFO`.
        // Or without a timestamp, i.e. `{"severity":"INFO`.
        bufs[0] = IoSlice::new(timestamp(buf));
        bufs[1] = IoSlice::new(b"\"severity\":\"");
        if record.level() == log::Level::Error && record.target() == PANIC_TARGET {
            // If we're panicking we increase the severity to critical.
            bufs[2] = IoSlice::new(b"CRITICAL");
        } else {
            bufs[2] = IoSlice::new(severity(record.level()));
        }
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
        // Optional file, e.g.
        // `","sourceLocation":{"file":"some_file.rs","line":"123"}}`, and a line
        // end.
        let n = if add_loc {
            bufs[10] = IoSlice::new(b",\"sourceLocation\":{\"file\":\"");
            bufs[11] = IoSlice::new(record.file().unwrap_or("??").as_bytes());
            bufs[12] = IoSlice::new(b"\",\"line\":\"");
            bufs[13] = IoSlice::new(line(buf));
            bufs[14] = IoSlice::new(b"\"}}\n");
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
fn write_timestamp(buf: &mut Buffer) {
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
fn timestamp(buf: &Buffer) -> &[u8] {
    &buf.buf[..TS_END_INDEX]
}

#[inline]
const fn severity(level: log::Level) -> &'static [u8] {
    // NOTE: gcloud doesn't have trace messages so we use debug twice.
    const SEVERITIES: [&[u8]; 6] = [b"OFF", b"ERROR", b"WARNING", b"INFO", b"DEBUG", b"DEBUG"];
    SEVERITIES[level as usize]
}

#[inline]
fn write_msg(buf: &mut Buffer, args: &fmt::Arguments) {
    buf.buf.truncate(TS_END_INDEX);
    if let Some(msg) = args.as_str() {
        json::Buf(&mut buf.buf)
            .write_str(msg)
            .unwrap_or_else(|_| unreachable!());
    } else {
        write!(json::Buf(&mut buf.buf), "{args}").unwrap_or_else(|_| unreachable!());
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
    let mut visitor = json::KeyValueVisitor(&mut buf.buf);
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
    let mut itoa = itoa::Buffer::new();
    buf.buf.extend_from_slice(itoa.format(line).as_bytes());
    buf.indices[2] = buf.buf.len();
}

#[inline]
fn line(buf: &Buffer) -> &[u8] {
    &buf.buf[buf.indices[1]..buf.indices[2]]
}
