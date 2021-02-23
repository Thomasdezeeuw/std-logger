use std::fmt;
use std::io::IoSlice;
use std::io::Write;

use log::{kv, Record};

/// Number of buffers the [`record`] function requires.
pub(crate) const BUFS_SIZE: usize = 11;

/// Formats a log `record`.
///
/// This writes into the buffer `buf` for things that need formatting, which it
/// resets itself. The returned slices is based on `bufs`, which is used to
/// order the writable buffers.
///
/// If `debug` is `true` the file and line are added.
#[inline]
pub(crate) fn record<'b>(
    bufs: &'b mut [IoSlice<'b>; BUFS_SIZE],
    buf: &'b mut Buffer,
    record: &'b Record,
    debug: bool,
) -> &'b [IoSlice<'b>] {
    // Write all parts of the buffer that need formatting.
    #[cfg(feature = "timestamp")]
    buf.write_ts();
    buf.write_msg(record.args());
    buf.write_key_values(record.key_values());
    if debug {
        buf.write_line(record.line().unwrap_or(0));
    }

    // Now that we've written the message to our buffer we have to construct it.
    // The first part of the message is the timestamp and log level, e.g.
    // `ts="2020-12-31T12:32:23.906132Z" lvl="INFO`.
    // Or without a timestamp, i.e. `lvl="INFO`.
    bufs[0] = IoSlice::new(buf.ts());
    bufs[1] = IoSlice::new(record.level().as_str().as_bytes());
    // The message (and the end of the log level), e.g. `" msg="some message`.
    bufs[2] = IoSlice::new(buf.msg());
    // The target, e.g. `" target="request`.
    bufs[3] = IoSlice::new(b"\" target=\"");
    bufs[4] = IoSlice::new(record.target().as_bytes());
    // The module, e.g. `" module="stored::http`.
    bufs[5] = IoSlice::new(b"\" module=\"");
    bufs[6] = IoSlice::new(record.module_path().unwrap_or("").as_bytes());
    // Any key value pairs supplied by the user.
    bufs[7] = IoSlice::new(buf.key_values());
    // Optional file, e.g. ` file="some_file:123"`, and a line end.
    let n = if debug {
        bufs[8] = IoSlice::new(b" file=\"");
        bufs[9] = IoSlice::new(record.file().unwrap_or("??").as_bytes());
        bufs[10] = IoSlice::new(buf.line());
        BUFS_SIZE
    } else {
        bufs[8] = IoSlice::new(b"\n");
        BUFS_SIZE - 2
    };

    &bufs[..n]
}

/// Number of indices used in `Buffer`:
/// 0) Message.
/// 1) Key value pairs.
/// 2) File line.
const N_INDICES: usize = 3;

/// Parts of the message we can reuse.
#[cfg(feature = "timestamp")]
const REUSABLE_PARTS: &[u8] = b"ts=\"0000-00-00T00:00:00.000000Z\" lvl=\"\" msg=\"";
#[cfg(not(feature = "timestamp"))]
const REUSABLE_PARTS: &[u8] = b"lvl=\"\" msg=\"";

/// Index of the end of `ts="..." lvl="`.
#[cfg(feature = "timestamp")]
const TS_END_INDEX: usize = 38;
#[cfg(not(feature = "timestamp"))]
const TS_END_INDEX: usize = 5;
/// Index where the message should be written to.
const MSG_START_INDEX: usize = TS_END_INDEX + 7;

pub(crate) struct Buffer {
    buf: Vec<u8>,
    indices: [usize; N_INDICES],
}

impl Buffer {
    pub(crate) fn new() -> Buffer {
        let mut buf = Vec::with_capacity(1024);
        // Write the parts of output that can be reused.
        buf.extend_from_slice(REUSABLE_PARTS);
        let indices = [0; N_INDICES];
        Buffer { buf, indices }
    }

    #[inline]
    #[cfg(feature = "timestamp")]
    fn write_ts(&mut self) {
        format_timestamp(&mut self.buf[..TS_END_INDEX]);
    }

    #[inline]
    fn ts(&self) -> &[u8] {
        &self.buf[..TS_END_INDEX]
    }

    #[inline]
    fn write_msg(&mut self, args: &fmt::Arguments) {
        self.buf.truncate(MSG_START_INDEX);
        #[cfg(not(feature = "nightly"))]
        write!(self.buf, "{}", args).unwrap_or_else(|_| unreachable!());
        #[cfg(feature = "nightly")]
        if let Some(msg) = args.as_str() {
            self.buf.extend_from_slice(msg.as_bytes());
        } else {
            write!(self.buf, "{}", args).unwrap_or_else(|_| unreachable!());
        }
        self.indices[0] = self.buf.len();
    }

    #[inline]
    fn msg(&self) -> &[u8] {
        // NOTE: not using `MSG_START_INDEX` here because we need to include the
        // `" msg="` format part.
        &self.buf[TS_END_INDEX..self.indices[0]]
    }

    #[inline]
    fn write_key_values(&mut self, kvs: &dyn kv::Source) {
        self.buf.extend_from_slice(b"\"");
        // TODO: see if we can add to the slice of `IoSlice` using the keys
        // and string values.
        let mut visitor = KeyValueVisitor(&mut self.buf);
        kvs.visit(&mut visitor).unwrap_or_else(|_| unreachable!());
        self.indices[1] = self.buf.len();
    }

    #[inline]
    fn key_values(&self) -> &[u8] {
        &self.buf[self.indices[0]..self.indices[1]]
    }

    #[inline]
    fn write_line(&mut self, line: u32) {
        self.buf.push(b':');
        let mut itoa = itoa::Buffer::new();
        self.buf.extend_from_slice(itoa.format(line).as_bytes());
        self.buf.extend_from_slice(b"\"\n");
        self.indices[2] = self.buf.len();
    }

    #[inline]
    fn line(&self) -> &[u8] {
        &self.buf[self.indices[1]..self.indices[2]]
    }
}

/// Format the timestamp in the following format:
/// `ts="YYYY-MM-DDThh:mm:ss.SSSSSSZ"`. For example:
/// `ts="2020-12-31T11:00:01.743357Z"`.
///
/// # Notes
///
/// The `buf` must come from [`Buffer::ts`] as it only overwrites the date, not
/// the format.
#[inline]
#[cfg(feature = "timestamp")]
fn format_timestamp(buf: &mut [u8]) {
    use std::mem::MaybeUninit;
    use std::time::{Duration, SystemTime};

    let now = SystemTime::now();
    let diff = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::new(0, 0));

    let mut tm = MaybeUninit::uninit();
    let secs_since_epoch = diff.as_secs() as i64;
    let tm = unsafe { libc::gmtime_r(&secs_since_epoch, tm.as_mut_ptr()) };
    let (year, month, day, hour, min, sec) = match unsafe { tm.as_ref() } {
        Some(tm) => (
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec,
        ),
        None => (0, 0, 0, 0, 0, 0),
    };
    let micros = diff.subsec_micros();

    let mut itoa = itoa::Buffer::new();
    buf[4..8].copy_from_slice(itoa.format(year).as_bytes());
    zero_pad2(&mut buf[9..11], itoa.format(month).as_bytes());
    zero_pad2(&mut buf[12..14], itoa.format(day).as_bytes());
    zero_pad2(&mut buf[15..17], itoa.format(hour).as_bytes());
    zero_pad2(&mut buf[18..20], itoa.format(min).as_bytes());
    zero_pad2(&mut buf[21..23], itoa.format(sec).as_bytes());
    zero_pad6(&mut buf[24..30], itoa.format(micros).as_bytes());
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
            if value {
                self.0.extend_from_slice(b"true");
            } else {
                self.0.extend_from_slice(b"false");
            }
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
