use std::collections::HashMap;
use std::io::{self, Read};
use std::time::{Duration, SystemTime};

use log::Level;
use std_logger_parser::{parse, ParseErrorKind, Record, Value};

const BUF_SIZE: usize = 4096;

#[track_caller]
fn test_parser(logs: &[u8], expected: Vec<Record>) {
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

#[test]
fn smoke() {
    let logs = b"ts=\"2021-02-23T13:15:48.624447Z\" lvl=\"INFO\" msg=\"Hello world\" target=\"key_value\" module=\"key_value\"\n";
    let expected = vec![new_record(
        Some(new_timestamp("2021-02-23T13:15:48.624447Z")),
        Level::Info,
        "Hello world",
        "key_value",
        Some("key_value"),
        None,
        HashMap::new(),
    )];
    test_parser(logs, expected);
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
    test_parser(logs, expected);
}

#[test]
fn ignore_whitespace() {
    let logs = b"  \n\n  \n";
    let expected = Vec::new();
    test_parser(logs, expected);
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
    test_parser(&logs, expected);
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
    test_parser(&logs, expected);
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
    test_parser(&logs, expected);
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
