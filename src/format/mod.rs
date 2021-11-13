pub(crate) mod logfmt;

/// Number of buffers the format functions require.
pub(crate) const BUFS_SIZE: usize = 11;

/// Number of indices used in `Buffer`:
/// 0) Message.
/// 1) Key value pairs.
/// 2) File line.
const N_INDICES: usize = 3;

/// Formatting buffer.
pub(crate) struct Buffer {
    buf: Vec<u8>,
    indices: [usize; N_INDICES],
}

impl Buffer {
    /// Create a new format `Buffer`.
    ///
    /// `reusable_parts` MUST be the `REUSABLE_PARTS` constant.
    pub(crate) fn new(reusable_parts: &[u8]) -> Buffer {
        let mut buf = Vec::with_capacity(1024);
        // Write the parts of output that can be reused.
        buf.extend_from_slice(reusable_parts);
        let indices = [0; N_INDICES];
        Buffer { buf, indices }
    }
}

// Allow access as `format::logfmt`.
pub(crate) use logfmt::format as logfmt;

/// Format the timestamp in the following format:
/// `YYYY-MM-DDThh:mm:ss.SSSSSSZ`. For example:
/// `2020-12-31T11:00:01.743357Z`.
///
/// # Notes
///
/// The `buf` must come from [`Buffer::ts`] as it only overwrites the date, not
/// the format.
#[inline]
#[cfg(feature = "timestamp")]
fn format_timestamp(buf: &mut [u8]) {
    let timestamp = crate::timestamp::Timestamp::now();
    let mut itoa = itoa::Buffer::new();
    buf[0..4].copy_from_slice(itoa.format(timestamp.year).as_bytes());
    zero_pad2(&mut buf[5..7], itoa.format(timestamp.month).as_bytes());
    zero_pad2(&mut buf[8..10], itoa.format(timestamp.day).as_bytes());
    zero_pad2(&mut buf[11..13], itoa.format(timestamp.hour).as_bytes());
    zero_pad2(&mut buf[14..16], itoa.format(timestamp.min).as_bytes());
    zero_pad2(&mut buf[17..19], itoa.format(timestamp.sec).as_bytes());
    zero_pad6(&mut buf[20..26], itoa.format(timestamp.micro).as_bytes());
}

#[inline]
#[cfg(feature = "timestamp")]
fn zero_pad2(buf: &mut [u8], v: &[u8]) {
    debug_assert_eq!(buf.len(), 2);
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
    debug_assert_eq!(buf.len(), 6);
    let start = 6 - v.len();
    for b in buf.iter_mut().take(start) {
        *b = b'0';
    }
    buf[start..6].copy_from_slice(v);
}
