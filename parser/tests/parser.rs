use std::collections::HashMap;
use std::io::{self, Read};
use std::time::{Duration, SystemTime};

use log::Level;
use std_logger_parser::{parse, ParseErrorKind, Record, Value};

const BUF_SIZE: usize = 4096;

#[track_caller]
fn test_parser<R: Read>(logs: R, expected: Vec<Record>) {
    let mut got = parse(logs);
    let mut expected = expected.into_iter();
    loop {
        match (got.next(), expected.next()) {
            (Some(got), Some(expected)) => {
                assert_eq!(got.expect("unexpected parsing error"), expected)
            }
            (Some(got), None) => panic!("unexpected additional record: {:?}", got),
            (None, Some(record)) => {
                panic!("missing records: {:?}, {:?}", record, expected.as_slice())
            }
            (None, None) => break,
        }
    }
}

fn new_record(
    timestamp: Option<SystemTime>,
    level: Level,
    msg: &str,
    target: &str,
    module: Option<&str>,
    file: Option<(&str, u32)>,
    key_values: HashMap<String, Value>,
) -> Record {
    let mut record = Record::empty();
    record.timestamp = timestamp;
    record.level = level;
    record.msg = msg.to_owned();
    record.target = target.to_owned();
    record.module = module.map(|m| m.to_owned());
    record.file = file.map(|(f, l)| (f.to_owned(), l));
    record.key_values = key_values;
    record
}

#[track_caller]
fn new_timestamp(ts: &str) -> SystemTime {
    let mut tm = libc::tm {
        tm_sec: ts[17..19].parse().unwrap(),
        tm_min: ts[14..16].parse().unwrap(),
        tm_hour: ts[11..13].parse().unwrap(),
        tm_mday: ts[8..10].parse().unwrap(),
        tm_mon: (ts[5..7].parse::<i32>().unwrap()) - 1,
        tm_year: (ts[0..4].parse::<i32>().unwrap()) - 1900,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null_mut(),
    };
    let time_offset = unsafe { libc::timegm(&mut tm) };
    // Create the timestamp from the time offset and the nanosecond precision.
    let nanos: u32 = ts[20..26].parse().unwrap();
    SystemTime::UNIX_EPOCH + Duration::new(time_offset as u64, nanos)
}

struct MultiSlice<'a> {
    slices: &'a mut [&'a [u8]],
}

impl<'a> Read for MultiSlice<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut remove = 0;
        let mut accumulated_len = 0;
        for slice in self.slices.iter_mut() {
            match slice.read(&mut buf[accumulated_len..]) {
                Ok(n) => {
                    accumulated_len += n;
                    if n == (&*slice).len() {
                        remove += 1;
                    } else {
                        break;
                    }
                }
                Err(err) => return Err(err),
            }
        }

        let slices = std::mem::replace(&mut self.slices, &mut []);
        self.slices = &mut slices[remove..];
        return Ok(accumulated_len);
    }
}

#[test]
fn smoke() {
    #[rustfmt::skip]
    let lines: &mut [&[u8]] = &mut [
        // Qouted values.
        b"ts=\"2021-02-23T13:15:48.624447Z\" lvl=\"INFO\" msg=\"Hello world\" target=\"target\" module=\"module\"\n",
        // Naked values.
        b"ts=2021-02-23T13:15:48.624447Z lvl=INFO msg=Hello target=target module=module\n",
        // File.
        b"ts=2021-02-23T13:15:48.624447Z lvl=warn msg=with_file target=key_value module=key_value file=some_file.rs:123\n",
        // No module.
        b"ts=2021-02-23T13:15:48.624447Z lvl=Error msg=\"No module\" target=target\n",
        // Nested qoute.
        b"ts=2021-02-23T13:15:48.624447Z lvl=TraCE msg=\"Some \"great\" message\" target=  target  \n",
        // Key-value pairs.
        b"ts=2021-02-23T13:15:48.624447Z lvl=DEBUG msg=\"key value pairs\" target=key_value module=key_value key1=value1 \"key2\"=\"value2\" key3  = 3  key4=-4 \"key5\"   = 5.0   key6=true key7=false\n",
        // Panic with backtrace, multi-line qouted values.
        b"ts=\"2021-02-23T13:16:09.576227Z\" lvl=\"ERROR\" msg=\"thread 'main' panicked at 'oops', examples/panic.rs:15\" target=\"panic\" module=\"\" backtrace=\"   0: std::backtrace_rs::backtrace::libunwind::trace\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/../../backtrace/src/backtrace/libunwind.rs:90:5\n      std::backtrace_rs::backtrace::trace_unsynchronized\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/../../backtrace/src/backtrace/mod.rs:66:5\n      std::backtrace::Backtrace::create\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/backtrace.rs:327:13\n   1: std::backtrace::Backtrace::force_capture\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/backtrace.rs:310:9\n   2: std_logger::log_panic\n             at ./src/lib.rs:346:21\n   3: core::ops::function::Fn::call\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/core/src/ops/function.rs:70:5\n   4: std::panicking::rust_panic_with_hook\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/panicking.rs:595:17\n   5: std::panicking::begin_panic::{{closure}}\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/std/src/panicking.rs:520:9\n   6: std::sys_common::backtrace::__rust_end_short_backtrace\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/std/src/sys_common/backtrace.rs:141:18\n   7: std::panicking::begin_panic\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/std/src/panicking.rs:519:12\n   8: panic::main\n             at ./examples/panic.rs:15:5\n   9: core::ops::function::FnOnce::call_once\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/core/src/ops/function.rs:227:5\n  10: std::sys_common::backtrace::__rust_begin_short_backtrace\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/std/src/sys_common/backtrace.rs:125:18\n  11: std::rt::lang_start::{{closure}}\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/std/src/rt.rs:66:18\n  12: core::ops::function::impls::<impl core::ops::function::FnOnce<A> for &F>::call_once\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/core/src/ops/function.rs:259:13\n      std::panicking::try::do_call\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/panicking.rs:379:40\n      std::panicking::try\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/panicking.rs:343:19\n      std::panic::catch_unwind\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/panic.rs:431:14\n      std::rt::lang_start_internal\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/rt.rs:51:25\n  13: std::rt::lang_start\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/std/src/rt.rs:65:5\n  14: _main\n\"\n",
        // No timestamp.
        b"lvl=\"INFO\" msg=\"Hello world\" \n",
    ];

    let expected = vec![
        new_record(
            Some(new_timestamp("2021-02-23T13:15:48.624447Z")),
            Level::Info,
            "Hello world",
            "target",
            Some("module"),
            None,
            HashMap::new(),
        ),
        new_record(
            Some(new_timestamp("2021-02-23T13:15:48.624447Z")),
            Level::Info,
            "Hello",
            "target",
            Some("module"),
            None,
            HashMap::new(),
        ),
        new_record(
            Some(new_timestamp("2021-02-23T13:15:48.624447Z")),
            Level::Warn,
            "with_file",
            "key_value",
            Some("key_value"),
            Some(("some_file.rs", 123)),
            HashMap::new(),
        ),
        new_record(
            Some(new_timestamp("2021-02-23T13:15:48.624447Z")),
            Level::Error,
            "No module",
            "target",
            None,
            None,
            HashMap::new(),
        ),
        new_record(
            Some(new_timestamp("2021-02-23T13:15:48.624447Z")),
            Level::Trace,
            "Some \"great\" message",
            "target",
            None,
            None,
            HashMap::new(),
        ),
        new_record(
            Some(new_timestamp("2021-02-23T13:15:48.624447Z")),
            Level::Debug,
            "key value pairs",
            "key_value",
            Some("key_value"),
            None,
            {
                let mut m = HashMap::new();
                m.insert("key1".to_owned(), Value::String("value1".to_owned()));
                m.insert("key2".to_owned(), Value::String("value2".to_owned()));
                m.insert("key3".to_owned(), Value::Int(3));
                m.insert("key4".to_owned(), Value::Int(-4));
                m.insert("key5".to_owned(), Value::Float(5.0));
                m.insert("key6".to_owned(), Value::Bool(true));
                m.insert("key7".to_owned(), Value::Bool(false));
                m
            },
        ),
        new_record(
            Some(new_timestamp("2021-02-23T13:16:09.576227Z")),
            Level::Error,
            "thread 'main' panicked at 'oops', examples/panic.rs:15",
            "panic",
            None,
            None,
            {
                let mut m = HashMap::new();
                m.insert("backtrace".to_owned(), Value::String("   0: std::backtrace_rs::backtrace::libunwind::trace\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/../../backtrace/src/backtrace/libunwind.rs:90:5\n      std::backtrace_rs::backtrace::trace_unsynchronized\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/../../backtrace/src/backtrace/mod.rs:66:5\n      std::backtrace::Backtrace::create\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/backtrace.rs:327:13\n   1: std::backtrace::Backtrace::force_capture\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/backtrace.rs:310:9\n   2: std_logger::log_panic\n             at ./src/lib.rs:346:21\n   3: core::ops::function::Fn::call\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/core/src/ops/function.rs:70:5\n   4: std::panicking::rust_panic_with_hook\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/panicking.rs:595:17\n   5: std::panicking::begin_panic::{{closure}}\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/std/src/panicking.rs:520:9\n   6: std::sys_common::backtrace::__rust_end_short_backtrace\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/std/src/sys_common/backtrace.rs:141:18\n   7: std::panicking::begin_panic\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/std/src/panicking.rs:519:12\n   8: panic::main\n             at ./examples/panic.rs:15:5\n   9: core::ops::function::FnOnce::call_once\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/core/src/ops/function.rs:227:5\n  10: std::sys_common::backtrace::__rust_begin_short_backtrace\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/std/src/sys_common/backtrace.rs:125:18\n  11: std::rt::lang_start::{{closure}}\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/std/src/rt.rs:66:18\n  12: core::ops::function::impls::<impl core::ops::function::FnOnce<A> for &F>::call_once\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/core/src/ops/function.rs:259:13\n      std::panicking::try::do_call\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/panicking.rs:379:40\n      std::panicking::try\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/panicking.rs:343:19\n      std::panic::catch_unwind\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/panic.rs:431:14\n      std::rt::lang_start_internal\n             at /rustc/a143517d44cac50b20cbd3a0b579addab40dd399/library/std/src/rt.rs:51:25\n  13: std::rt::lang_start\n             at /Users/thomas/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/std/src/rt.rs:65:5\n  14: _main\n".to_owned()));
                m
            },
        ),
        new_record(
            None,
            Level::Info,
            "Hello world",
            "",
            None,
            None,
            HashMap::new(),
        ),
    ];
    test_parser(MultiSlice { slices: lines }, expected);
}

#[test]
fn no_new_line() {
    let logs = b"ts=\"2021-02-23T13:15:48.624447Z\" lvl=\"INFO\" msg=\"Hello world\" target=\"key_value\" module=\"key_value\"";
    let expected = vec![new_record(
        Some(new_timestamp("2021-02-23T13:15:48.624447Z")),
        Level::Info,
        "Hello world",
        "key_value",
        Some("key_value"),
        None,
        HashMap::new(),
    )];
    test_parser::<&[u8]>(logs, expected);
}

#[test]
fn ignore_whitespace() {
    let logs = b"  \n\n  \n";
    let expected = Vec::new();
    test_parser::<&[u8]>(logs, expected);
}

#[test]
fn partial_input_key() {
    let mut logs = Vec::with_capacity(BUF_SIZE + 200);
    // Make sure the log is partially read, split the `msg` key.
    logs.resize(BUF_SIZE - 46, b' ');
    logs.extend_from_slice(b"ts=\"2021-02-23T13:15:48.624447Z\" lvl=\"INFO\" msg=\"Hello world\" target=\"key_value\" module=\"key_value\"");
    assert_eq!(&logs[BUF_SIZE - 2..BUF_SIZE], b"ms");

    let expected = vec![new_record(
        Some(new_timestamp("2021-02-23T13:15:48.624447Z")),
        Level::Info,
        "Hello world",
        "key_value",
        Some("key_value"),
        None,
        HashMap::new(),
    )];
    test_parser(&*logs, expected);
}

#[test]
fn partial_input_value() {
    let mut logs = Vec::with_capacity(BUF_SIZE + 200);
    // Make sure the log is partially read, split the `msg` value.
    logs.resize(BUF_SIZE - 59, b' ');
    logs.extend_from_slice(b"ts=\"2021-02-23T13:15:48.624447Z\" lvl=\"INFO\" msg=\"Hello world\" target=\"key_value\" module=\"key_value\"");
    assert_eq!(&logs[BUF_SIZE - 4..BUF_SIZE], b"worl");

    let expected = vec![new_record(
        Some(new_timestamp("2021-02-23T13:15:48.624447Z")),
        Level::Info,
        "Hello world",
        "key_value",
        Some("key_value"),
        None,
        HashMap::new(),
    )];
    test_parser(&*logs, expected);
}

#[test]
fn buffer_too_small() {
    let mut logs = Vec::with_capacity(BUF_SIZE + 200);
    let msg = "a".repeat(BUF_SIZE);
    logs.extend_from_slice(
        b"ts=\"2021-02-23T13:15:48.624447Z\" lvl=\"INFO\" target=\"test\" msg=\"",
    );
    logs.extend_from_slice(msg.as_bytes());
    logs.extend_from_slice(b"\"");
    let expected = vec![new_record(
        Some(new_timestamp("2021-02-23T13:15:48.624447Z")),
        Level::Info,
        &msg,
        "test",
        None,
        None,
        HashMap::new(),
    )];
    test_parser(&*logs, expected);
}

#[test]
fn invalid_lines() {
    #[rustfmt::skip]
    let lines: &mut [&[u8]] = &mut [
        // Invalid key.
        &[115, 111, 109, 101, 0x80, 107, 101, 121, 61, 49, 50, 51, 10], // Invalid UTF-8.

        // Invalid timestamp.
        b"ts=2021-02-23T13:15:48.62444Z\n", // Invalid length (too short).
        b"ts=2021-02-23T13:15:48.624447ZA\n", // Invalid length (too long).
        // Incorrect formatting of delimiters.
        b"ts=2021A02-23T13:15:48.624447Z\n", // Year-month.
        b"ts=2021-02A23T13:15:48.624447Z\n", // Month-day.
        b"ts=2021-02-23A13:15:48.624447Z\n", // Date-time.
        b"ts=2021-02-23T13A15:48.624447Z\n", // Hour:minute.
        b"ts=2021-02-23T13:15A48.624447Z\n", // Minute:second.
        b"ts=2021-02-23T13:15:48A624447Z\n", // Second.nanosecond.
        b"ts=2021-02-23T13:15:48.624447A\n", // Timezone.
        &[116, 115, 61, 0x80, 48, 50, 49, 45, 48, 50, 45, 50, 51, 84, 49, 51, 58, 49, 53, 58, 52, 56, 46, 54, 50, 52, 52, 52, 55, 90, 10], // Invalid UTF-8.
        // Invalid numbers.
        b"ts=A021-02-23T13:15:48.624447Z\n", // Year.
        b"ts=2021-A2-23T13:15:48.624447Z\n", // Month.
        b"ts=2021-02-A3TA3:15:48.624447Z\n", // Day.
        b"ts=2021-02-23TA3:15:48.624447Z\n", // Hour.
        b"ts=2021-02-23T13:A5:48.624447Z\n", // Minute.
        b"ts=2021-02-23T13:15:A8.624447Z\n", // Second.
        b"ts=2021-02-23T13:15:48.A24447Z\n", // Nanosecond.

        // Invalid level.
        b"lvl=NOT_INFO\n", // Not a level.
        &[108, 118, 108, 61, 0x80, 79, 84, 95, 73, 78, 70, 79, 10], // Invalid UTF-8.
        // Invalid key.
        &[107, 101, 121, 61, 0x80, 97, 108, 117, 101, 10], // Invalid UTF-8.

        // Invalid file.
        &[102, 105, 108, 101, 61, 0x80, 111, 109, 101, 95, 102, 105, 108, 101, 46, 114, 115, 58, 49, 48, 10], // Invalid UTF-8.
        b"file=some_file.rs10\n", // Missing colon (`:`).
        b"file=some_file.rs:A0\n", // Invalid line number.

        // Invalid msg value.
        &[109, 115, 103, 61, 0x80, 111, 109, 101, 95, 109, 101, 115, 115, 97, 103, 101, 10],
        // Invalid target value.
        &[116, 97, 114, 103, 101, 116, 61, 0x80, 97, 114, 103, 101, 116, 10],
        // Invalid module value.
        &[109, 111, 100, 117, 108, 101, 61, 0x80, 111, 100, 117, 108, 101, 10],
        // Invalid key value.
        &[107, 101, 121, 61, 0x80, 97, 108, 117, 101, 10],
    ];

    // I know... it's not great.
    let expected = &[
        ParseErrorKind::KeyInvalidUt8,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidTimestamp,
        ParseErrorKind::InvalidLevel,
        ParseErrorKind::InvalidLevel,
        ParseErrorKind::InvalidValue,
        ParseErrorKind::InvalidFile,
        ParseErrorKind::InvalidFile,
        ParseErrorKind::InvalidFile,
        ParseErrorKind::InvalidValue,
        ParseErrorKind::InvalidValue,
        ParseErrorKind::InvalidValue,
        ParseErrorKind::InvalidValue,
    ];

    assert_eq!(lines.len(), expected.len());
    let expected = lines
        .iter()
        .zip(expected.iter())
        .map(|(l, e)| (Some((&l[..l.len() - 1]).into()), e))
        .collect::<Vec<_>>();

    let logs = MultiSlice { slices: lines };
    let mut expected = expected.into_iter();
    for record in parse(logs) {
        let err = record.unwrap_err();
        let expected = expected.next().unwrap();
        assert_eq!(
            err.line,
            expected.0,
            "got: {}, expected: {}",
            String::from_utf8_lossy(&*err.line.as_ref().unwrap()),
            String::from_utf8_lossy(&*expected.0.as_ref().unwrap()),
        );
        assert_eq!(err.kind, *expected.1);
    }
    assert!(expected.len() == 0, "left: {:?}", expected.as_slice());
}

#[test]
fn io_error_and_continue() {
    struct ErrReading<'a> {
        first: &'a [u8],
        err: Option<io::ErrorKind>,
        second: &'a [u8],
    }

    impl<'a> Read for ErrReading<'a> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if !self.first.is_empty() {
                self.first.read(buf)
            } else if let Some(err) = self.err.take() {
                Err(err.into())
            } else {
                self.second.read(buf)
            }
        }
    }

    let logs = ErrReading {
        first: b"lvl=INFO msg=Hello target=",
        err: Some(io::ErrorKind::WouldBlock),
        second: b"target",
    };

    let mut parser = parse(logs);
    match parser.next() {
        Some(Err(err)) => match err.kind {
            ParseErrorKind::Io(ref io_err) if io_err.kind() == io::ErrorKind::WouldBlock => {
                assert!(err.line.is_none());
            }
            _ => panic!("unexpected error: {}", err),
        },
        _ => panic!("unexpected result"),
    }
    let got = parser.next().unwrap().unwrap();
    let expected = new_record(
        None,
        Level::Info,
        "Hello",
        "target",
        None,
        None,
        HashMap::new(),
    );
    assert_eq!(got, expected);
    assert!(parser.next().is_none());
}
