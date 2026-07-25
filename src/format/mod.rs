use log::{kv, Record};

pub(crate) mod logfmt;
pub(crate) use logfmt::LogFmt;

pub(crate) mod json;
pub(crate) use json::Json;

pub(crate) mod gcloud;
pub(crate) use gcloud::Gcloud;

/// Trait that defines how to format a [`log::Record`].
pub trait Format {
    /// Formats a log `record`.
    ///
    /// This writes into the buffer `buf` for things that need formatting, which
    /// it resets itself. The returned slices is based on `bufs`, which is used
    /// to order the writable buffers.
    ///
    /// If `add_loc` is `true` the file and line are added.
    fn format<'b, Kvs: kv::Source>(
        buf: &'b mut Vec<u8>,
        record: &'b Record,
        kvs: &Kvs,
        add_loc: bool,
    );
}

/// Format the timestamp in the following format:
/// `YYYY-MM-DDThh:mm:ss.SSSSSSZ`. For example:
/// `2020-12-31T11:00:01.743357Z`.
#[inline]
#[cfg(feature = "timestamp")]
fn format_timestamp(buf: &mut [u8]) {
    let _ = buf[26];
    let timestamp = crate::timestamp::Timestamp::now();
    let mut itoa = itoa::Buffer::new();
    buf[0..4].copy_from_slice(itoa.format(timestamp.year).as_bytes());
    buf[4] = b'-';
    zero_pad2(&mut buf[5..], itoa.format(timestamp.month).as_bytes());
    buf[7] = b'-';
    zero_pad2(&mut buf[8..], itoa.format(timestamp.day).as_bytes());
    buf[10] = b'T';
    zero_pad2(&mut buf[11..], itoa.format(timestamp.hour).as_bytes());
    buf[13] = b':';
    zero_pad2(&mut buf[14..], itoa.format(timestamp.min).as_bytes());
    buf[16] = b':';
    zero_pad2(&mut buf[17..], itoa.format(timestamp.sec).as_bytes());
    buf[19] = b'.';
    zero_pad6(&mut buf[20..], itoa.format(timestamp.micro).as_bytes());
    buf[26] = b'Z';
}

#[inline]
#[cfg(feature = "timestamp")]
fn zero_pad2(buf: &mut [u8], v: &[u8]) {
    let _ = buf[1];
    if v.len() == 1 {
        buf[0] = b'0';
        buf[1] = v[0];
    } else {
        buf[0] = v[0];
        buf[1] = v[1];
    }
}

#[inline]
#[cfg(feature = "timestamp")]
fn zero_pad6(buf: &mut [u8], v: &[u8]) {
    let _ = buf[5];
    let start = 6 - v.len();
    for b in buf.iter_mut().take(start) {
        *b = b'0';
    }
    buf[start..6].copy_from_slice(v);
}
