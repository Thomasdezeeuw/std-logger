//! Google Cloud Platform structured logging using JSON, following
//! <https://cloud.google.com/logging/docs/structured-logging>.

use std::io::IoSlice;

use log::{kv, Record};

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
        json::write_timestamp(buf);
        json::write_msg(buf, record.args());
        json::write_key_values(buf, record.key_values(), kvs);
        if add_loc {
            json::write_line(buf, record.line().unwrap_or(0));
        }

        // Now that we've written the message to our buffer we have to construct it.
        // The first part of the message is the timestamp and log level (severity),
        // e.g. `{"timestamp":"2020-12-31T12:32:23.906132Z","severity":"INFO`.
        // Or without a timestamp, i.e. `{"severity":"INFO`.
        bufs[0] = IoSlice::new(json::timestamp(buf));
        bufs[1] = IoSlice::new(b"\"severity\":\"");
        if record.level() == log::Level::Error && record.target() == PANIC_TARGET {
            // If we're panicking we increase the severity to critical.
            bufs[2] = IoSlice::new(b"CRITICAL");
        } else {
            bufs[2] = IoSlice::new(severity(record.level()));
        }
        // The message (and the end of the log level), e.g. `","message":"some message`.
        bufs[3] = IoSlice::new(b"\",\"message\":\"");
        bufs[4] = IoSlice::new(json::msg(buf));
        // The target, e.g. `","target":"request`.
        bufs[5] = IoSlice::new(b"\",\"target\":\"");
        bufs[6] = IoSlice::new(record.target().as_bytes());
        // The module, e.g. `","module":"stored::http`.
        bufs[7] = IoSlice::new(b"\",\"module\":\"");
        bufs[8] = IoSlice::new(record.module_path().unwrap_or("").as_bytes());
        // Any key value pairs supplied by the user.
        bufs[9] = IoSlice::new(json::key_values(buf));
        // Optional file, e.g.
        // `","sourceLocation":{"file":"some_file.rs","line":"123"}}`, and a line
        // end.
        let n = if add_loc {
            bufs[10] = IoSlice::new(b",\"sourceLocation\":{\"file\":\"");
            bufs[11] = IoSlice::new(record.file().unwrap_or("??").as_bytes());
            bufs[12] = IoSlice::new(b"\",\"line\":\"");
            bufs[13] = IoSlice::new(json::line(buf));
            bufs[14] = IoSlice::new(b"\"}}\n");
            15
        } else {
            bufs[10] = IoSlice::new(b"}\n");
            11
        };
        &bufs[..n]
    }
}

#[inline]
const fn severity(level: log::Level) -> &'static [u8] {
    // NOTE: gcloud doesn't have trace messages so we use debug twice.
    const SEVERITIES: [&[u8]; 6] = [b"OFF", b"ERROR", b"WARNING", b"INFO", b"DEBUG", b"DEBUG"];
    SEVERITIES[level as usize]
}
