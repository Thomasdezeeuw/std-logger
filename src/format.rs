use std::fmt;
use std::io::Write;

use log::{kv, Record};

use crate::REQUEST_TARGET;

/// Formats `record`, writing into `buf`.
#[inline(always)]
pub(crate) fn record(buf: &mut Vec<u8>, record: &Record) {
    #[cfg(feature = "timestamp")]
    format_timestamp(buf);

    match record.target() {
        REQUEST_TARGET => {
            buf.extend_from_slice(b"[REQUEST] ");
            buf.extend_from_slice(record.module_path().unwrap_or("").as_bytes())
        }
        target => {
            buf.push(b'[');
            // TODO: replace with `Level::as_str`.
            write!(buf, "{}", record.level()).unwrap_or_else(|_| unreachable!());
            buf.push(b']');
            buf.push(b' ');
            buf.extend_from_slice(target.as_bytes());
        }
    }

    buf.push(b':');
    buf.push(b' ');
    writeln!(
        buf,
        "{}{}",
        record.args(),
        KeyValuePrinter(record.key_values())
    )
    .unwrap_or_else(|_| unreachable!());
}

#[cfg(feature = "timestamp")]
#[inline(always)]
fn format_timestamp(buf: &mut Vec<u8>) {
    use chrono::{Datelike, Timelike};

    let timestamp = chrono::Utc::now();
    write!(
        buf,
        "{:004}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z ",
        timestamp.year(),
        timestamp.month(),
        timestamp.day(),
        timestamp.hour(),
        timestamp.minute(),
        timestamp.second(),
        timestamp.nanosecond() / 1000,
    )
    .unwrap_or_else(|_| unreachable!());
}

/// Prints key values in ": key1=value1, key2=value2" format.
///
/// # Notes
///
/// Prints ": " itself, only when there is at least one key value pair.
struct KeyValuePrinter<'a>(&'a dyn kv::Source);

impl<'a> fmt::Display for KeyValuePrinter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0
            .visit(&mut KeyValueVisitor(true, f))
            .map_err(|_| fmt::Error)
    }
}

struct KeyValueVisitor<'a, 'b>(bool, &'a mut fmt::Formatter<'b>);

impl<'a, 'b, 'kvs> kv::Visitor<'kvs> for KeyValueVisitor<'a, 'b> {
    fn visit_pair(&mut self, key: kv::Key<'kvs>, value: kv::Value<'kvs>) -> Result<(), kv::Error> {
        self.1
            .write_str(if self.0 { ": " } else { ", " })
            .and_then(|()| {
                self.0 = false;
                write!(self.1, "{}={}", key, value)
            })
            .map_err(Into::into)
    }
}
