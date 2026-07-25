//! Google Cloud Platform structured logging using JSON, following
//! <https://cloud.google.com/logging/docs/structured-logging>.

use log::{kv, Record};

use crate::format::json::{self, TS_END_INDEX};
use crate::format::Format;
use crate::PANIC_TARGET;

/// Google Cloud Platform structured logging using JSON, following
/// <https://cloud.google.com/logging/docs/structured-logging>.
#[allow(missing_debug_implementations)]
pub enum Gcloud {}

impl Format for Gcloud {
    fn format<'b, Kvs: kv::Source>(
        buf: &'b mut Vec<u8>,
        record: &'b Record,
        kvs: &Kvs,
        add_loc: bool,
    ) {
        buf.truncate(TS_END_INDEX + 12);

        // The first part of the message is the timestamp, e.g.
        // `{"timestamp":"2020-12-31T12:32:23.906132Z`
        buf[0] = b'{';
        #[cfg(feature = "timestamp")]
        json::write_timestamp(buf);

        // Next is the log severity, e.g. `","severity":"INFO`.
        buf[TS_END_INDEX..TS_END_INDEX + 12].copy_from_slice(b"\"severity\":\"");
        if record.level() == log::Level::Error && record.target() == PANIC_TARGET {
            // If we're panicking we increase the severity to critical.
            buf.extend_from_slice(b"CRITICAL");
        } else {
            buf.extend_from_slice(severity(record.level()));
        }

        // The message (and the end of the log level), e.g. `","message":"some
        // message`.
        buf.extend_from_slice(b"\",\"message\":\"");
        json::write_msg(buf, record.args());

        // The target, e.g. `","target":"request`.
        buf.extend_from_slice(b"\",\"target\":\"");
        buf.extend_from_slice(record.target().as_bytes());

        // The module, e.g. `","module":"stored::http"`.
        buf.extend_from_slice(b"\",\"module\":\"");
        buf.extend_from_slice(record.module_path().unwrap_or("").as_bytes());
        buf.extend_from_slice(b"\"");

        // Any key value pairs supplied by the user.
        json::write_key_values(buf, record.key_values(), kvs);

        // Optional file, e.g.
        // `","sourceLocation":{"file":"some_file.rs","line":"123"}}`, and a line
        // end.
        if add_loc {
            buf.extend_from_slice(b",\"sourceLocation\":{\"file\":\"");
            buf.extend_from_slice(record.file().unwrap_or("??").as_bytes());
            buf.extend_from_slice(b"\",\"line\":\"");
            let mut itoa = itoa::Buffer::new();
            buf.extend_from_slice(itoa.format(record.line().unwrap_or(0)).as_bytes());
            buf.extend_from_slice(b"\"}}\n");
        } else {
            buf.extend_from_slice(b"}\n");
        }
    }
}

#[inline]
const fn severity(level: log::Level) -> &'static [u8] {
    // NOTE: gcloud doesn't have trace messages so we use debug twice.
    const SEVERITIES: [&[u8]; 6] = [b"OFF", b"ERROR", b"WARNING", b"INFO", b"DEBUG", b"DEBUG"];
    SEVERITIES[level as usize]
}
