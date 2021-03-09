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
