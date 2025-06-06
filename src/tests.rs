use std::io::{IoSlice, Write};
use std::mem::take;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use std::{env, fmt, panic, str};

use log::{debug, error, info, kv, trace, warn, Level, LevelFilter, Record};

use crate::config::{get_log_targets, get_max_level, NoKvs};
use crate::format::{self, Format, Gcloud, Json, LogFmt};
use crate::{request, Targets, BUFS_SIZE, LOG_OUTPUT, PANIC_TARGET, REQUEST_TARGET};

/// Macro to create a group of sequential tests.
macro_rules! sequential_tests {
    ( $(fn $name: ident () $body: block)+ ) => {
        /// A global lock for testing sequentially.
        static SEQUENTIAL_TEST_MUTEX: Mutex<()> = Mutex::new(());

        $(
        #[test]
        fn $name() {
            let guard = SEQUENTIAL_TEST_MUTEX.lock().unwrap();
            // Catch any panics to not poison the lock.
            if let Err(err) = panic::catch_unwind(|| $body) {
                drop(guard);
                panic::resume_unwind(err);
            }
        }
        )+
    };
}

sequential_tests! {
    fn should_get_the_correct_log_level_from_env() {
        let tests = &[
            ("LOG", "TRACE", LevelFilter::Trace),
            ("LOG", "ERROR", LevelFilter::Error),
            ("LOG_LEVEL", "ERROR", LevelFilter::Error),
            ("LOG_LEVEL", "DEBUG", LevelFilter::Debug),
            ("TRACE", "1", LevelFilter::Trace),
            ("DEBUG", "1", LevelFilter::Debug),
        ];

        for (env_var, env_val, want) in tests {
            env::set_var(env_var, env_val);

            let got = get_max_level();
            assert_eq!(*want, got);

            env::remove_var(env_var);
        }

        // Should default to info.
        env::remove_var("TRACE");
        env::remove_var("DEBUG");
        env::remove_var("LOG");
        env::remove_var("LOG_LEVEL");
        assert_eq!(get_max_level(), LevelFilter::Info);
    }

    fn should_get_correct_log_targets() {
        let tests = &[
            ("", Targets::All),
            ("crate1", Targets::Only(vec!["crate1".into()].into_boxed_slice())),
            ("crate1::mod1", Targets::Only(vec!["crate1::mod1".into()].into_boxed_slice())),
            ("crate1,crate2", Targets::Only(vec!["crate1".into(), "crate2".into()].into_boxed_slice())),
        ];

        for (env_val, want) in tests {
            env::set_var("LOG_TARGET", env_val);

            let got = get_log_targets();
            assert_eq!(*want, got);
        }

        env::remove_var("LOG_TARGET");
        assert_eq!(get_log_targets(), Targets::All);
    }

    fn log_output() {
        LOG_OUTPUT.lock().unwrap().clear();

        env::set_var("LOG_LEVEL", "TRACE");
        crate::Config::logfmt().init();
        env::remove_var("LOG_LEVEL");

        let want = &[
            "lvl=\"TRACE\" msg=\"trace message\" target=\"std_logger::tests\" module=\"std_logger::tests\" file=\"src/tests.rs:100\"\n",
            "lvl=\"DEBUG\" msg=\"debug message\" target=\"std_logger::tests\" module=\"std_logger::tests\" file=\"src/tests.rs:101\"\n",
            "lvl=\"INFO\" msg=\"info message\" target=\"std_logger::tests\" module=\"std_logger::tests\" file=\"src/tests.rs:102\"\n",
            "lvl=\"WARN\" msg=\"warn message\" target=\"std_logger::tests\" module=\"std_logger::tests\" file=\"src/tests.rs:103\"\n",
            "lvl=\"ERROR\" msg=\"error message\" target=\"std_logger::tests\" module=\"std_logger::tests\" file=\"src/tests.rs:104\"\n",
            "lvl=\"INFO\" msg=\"request message1\" target=\"request\" module=\"std_logger::tests\" file=\"src/tests.rs:105\"\n",
            "lvl=\"INFO\" msg=\"request message2\" target=\"request\" module=\"std_logger::tests\" file=\"src/tests.rs:106\"\n",
        ];

        #[cfg(feature = "timestamp")]
        let timestamp = SystemTime::now();

        trace!("trace message");
        debug!("debug message");
        info!("info message");
        warn!("warn message");
        error!("error message");
        info!(target: REQUEST_TARGET, "request message1");
        request!("request message2");

        // Make sure the panics aren't logged.
        let _ = std::panic::take_hook();
        let got = take(&mut *(LOG_OUTPUT.lock().unwrap()));

        let mut got_length = 0;
        for (want, got) in want.iter().zip(got.into_iter()) {
            let got = str::from_utf8(&got).expect("unable to parse string");

            #[allow(unused_mut)]
            let mut want = (*want).to_owned();
            #[cfg(feature = "timestamp")]
            { want = add_timestamp(want, timestamp, got); }

            assert_eq!(got, want.as_str(), "message differ");
            got_length += 1;
        }

        assert_eq!(got_length, want.len(), "the number of log messages got differs from the amount of messages wanted");
    }
}

fn add_timestamp(message: String, timestamp: SystemTime, got: &str) -> String {
    use std::mem::MaybeUninit;

    let diff = timestamp
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::new(0, 0));
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

    // Add the timestamp to the expected string.
    let micros = &got[24..30]; // We can never match the microseconds, so we just copy them.
    let timestamp =
        format!("ts=\"{year:004}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}.{micros}Z\"");
    format!("{timestamp} {message}")
}

#[test]
fn targets_should_log() {
    let targets = &[
        Targets::All,
        Targets::Only(vec!["crate1".into()].into_boxed_slice()),
        Targets::Only(vec!["crate1::mod1".into()].into_boxed_slice()),
        Targets::Only(vec!["crate1".into(), "crate2".into()].into_boxed_slice()),
    ];

    let tests = vec![
        ("", vec![true, false, false, false]),
        ("crate1", vec![true, true, false, true]),
        ("crate1::mod1", vec![true, true, true, true]),
        ("crate2", vec![true, false, false, true]),
        ("crate2::mod2", vec![true, false, false, true]),
        // Requests should always be logged.
        (REQUEST_TARGET, vec![true, true, true, true]),
        // Panics should always be logged.
        (PANIC_TARGET, vec![true, true, true, true]),
    ];

    for (test_target, wanted) in tests {
        for (target, want) in targets.iter().zip(wanted) {
            assert_eq!(
                target.should_log(test_target),
                want,
                "targets to log: {target:?}, logging target: {test_target}",
            )
        }
    }
}

struct MyDisplay;

impl fmt::Display for MyDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MyDisplay")
    }
}

#[test]
fn format_logfmt() {
    format_test::<LogFmt, _>(&[
        "lvl=\"INFO\" msg=\"some\\r\\n\\t\\nmessage\" target=\"some_target1\" module=\"module_path1\" key1=\"value1\" file=\"file1:123\"\n",
        "lvl=\"INFO\" msg=\"some\\r\\n\\t\\nmessage\" target=\"some_target1\" module=\"module_path1\" key1=\"value1\"\n",
        #[cfg(not(feature = "serde1"))]
        "lvl=\"WARN\" msg=\"arguments2 with \\\"quotes\\\"\" target=\"second_target\" module=\"module_path1\" key2a=\"value2\" key2b=123 key3c=-123 key3d=123.0 key2e=true key2f=false key2g=\"c\" key2\\\"g=\"MyDisplay\" null_key=null file=\"file2:111\"\n",
        #[cfg(feature = "serde1")]
        "lvl=\"WARN\" msg=\"arguments2 with \\\"quotes\\\"\" target=\"second_target\" module=\"module_path1\" key2a=\"value2\" key2b=123 key3c=-123 key3d=123.0 key2e=true key2f=false key2g=\"c\" key2\\\"g=\"MyDisplay\" null_key=null serde_map=\"MyValue { a: 1, b: \\\"2\\\", c: MyValue2 { d: 3.0 } }\" serde_array=\"[1, 2, 3]\" serde_tuple=\"(1, 2.0, \\\"3\\\")\" file=\"file2:111\"\n",
        "lvl=\"ERROR\" msg=\"panicking!\" target=\"panic\" module=\"\" file=\"??:0\"\n",
    ], add_timestamp);
}

#[test]
fn format_json() {
    format_test::<Json, _>(&[
        "{\"level\":\"INFO\",\"message\":\"some\\r\\n\\t\\nmessage\",\"target\":\"some_target1\",\"module\":\"module_path1\",\"key1\":\"value1\",\"file\":\"file1\",\"line\":\"123\"}\n",
        "{\"level\":\"INFO\",\"message\":\"some\\r\\n\\t\\nmessage\",\"target\":\"some_target1\",\"module\":\"module_path1\",\"key1\":\"value1\"}\n",
        #[cfg(not(feature = "serde1"))]
        "{\"level\":\"WARN\",\"message\":\"arguments2 with \\\"quotes\\\"\",\"target\":\"second_target\",\"module\":\"module_path1\",\"key2a\":\"value2\",\"key2b\":123,\"key3c\":-123,\"key3d\":123.0,\"key2e\":true,\"key2f\":false,\"key2g\":\"c\",\"key2\\\"g\":\"MyDisplay\",\"null_key\":null,\"file\":\"file2\",\"line\":\"111\"}\n",
        #[cfg(feature = "serde1")]
        "{\"level\":\"WARN\",\"message\":\"arguments2 with \\\"quotes\\\"\",\"target\":\"second_target\",\"module\":\"module_path1\",\"key2a\":\"value2\",\"key2b\":123,\"key3c\":-123,\"key3d\":123.0,\"key2e\":true,\"key2f\":false,\"key2g\":\"c\",\"key2\\\"g\":\"MyDisplay\",\"null_key\":null,\"serde_map\":{\"a\":1,\"b\":\"2\",\"c\":{\"d\":3.0}},\"serde_array\":[1,2,3],\"serde_tuple\":[1,2.0,\"3\"],\"file\":\"file2\",\"line\":\"111\"}\n",
        "{\"level\":\"ERROR\",\"message\":\"panicking!\",\"target\":\"panic\",\"module\":\"\",\"file\":\"??\",\"line\":\"0\"}\n",
    ], add_timestamp_json);
}

#[test]
fn format_gcloud() {
    format_test::<Gcloud, _>(&[
        "{\"severity\":\"INFO\",\"message\":\"some\\r\\n\\t\\nmessage\",\"target\":\"some_target1\",\"module\":\"module_path1\",\"key1\":\"value1\",\"sourceLocation\":{\"file\":\"file1\",\"line\":\"123\"}}\n",
        "{\"severity\":\"INFO\",\"message\":\"some\\r\\n\\t\\nmessage\",\"target\":\"some_target1\",\"module\":\"module_path1\",\"key1\":\"value1\"}\n",
        #[cfg(not(feature = "serde1"))]
        "{\"severity\":\"WARNING\",\"message\":\"arguments2 with \\\"quotes\\\"\",\"target\":\"second_target\",\"module\":\"module_path1\",\"key2a\":\"value2\",\"key2b\":123,\"key3c\":-123,\"key3d\":123.0,\"key2e\":true,\"key2f\":false,\"key2g\":\"c\",\"key2\\\"g\":\"MyDisplay\",\"null_key\":null,\"sourceLocation\":{\"file\":\"file2\",\"line\":\"111\"}}\n",
        #[cfg(feature = "serde1")]
        "{\"severity\":\"WARNING\",\"message\":\"arguments2 with \\\"quotes\\\"\",\"target\":\"second_target\",\"module\":\"module_path1\",\"key2a\":\"value2\",\"key2b\":123,\"key3c\":-123,\"key3d\":123.0,\"key2e\":true,\"key2f\":false,\"key2g\":\"c\",\"key2\\\"g\":\"MyDisplay\",\"null_key\":null,\"serde_map\":{\"a\":1,\"b\":\"2\",\"c\":{\"d\":3.0}},\"serde_array\":[1,2,3],\"serde_tuple\":[1,2.0,\"3\"],\"sourceLocation\":{\"file\":\"file2\",\"line\":\"111\"}}\n",
        "{\"severity\":\"CRITICAL\",\"message\":\"panicking!\",\"target\":\"panic\",\"module\":\"\",\"sourceLocation\":{\"file\":\"??\",\"line\":\"0\"}}\n",
    ], add_timestamp_json);
}

fn add_timestamp_json(want: String, timestamp: SystemTime, got: &str) -> String {
    let mut want = want.to_owned();
    let timestamp = add_timestamp(String::new(), timestamp, &got[10..]);
    let timestamp = format!("\"timestamp\":\"{}\",", &timestamp[4..timestamp.len() - 2]);
    want.insert_str(1, &timestamp);
    want
}

fn format_test<F, A>(expected: &[&str; 4], add_timestamp: A)
where
    F: Format,
    A: Fn(String, SystemTime, &str) -> String,
{
    let record1 = Record::builder()
        .args(format_args!("some\r\n\t\nmessage"))
        .level(Level::Info)
        .target("some_target1")
        .module_path_static(Some("module_path1"))
        .file_static(Some("file1"))
        .line(Some(123))
        .key_values(&("key1", "value1"))
        .build();
    #[cfg(feature = "serde1")]
    let vec = vec![1, 2, 3];
    let kvs: &[(&str, &dyn kv::ToValue)] = &[
        ("key2a", (&"value2") as &dyn kv::ToValue),
        ("key2b", &123u64),
        ("key3c", &-123i64),
        ("key3d", &123.0f64),
        ("key2e", &true),
        ("key2f", &false),
        ("key2g", &'c'),
        (
            "key2\"g",
            &(&log::kv::Value::from_display(&MyDisplay) as &dyn kv::ToValue),
        ),
        ("null_key", &None::<&str>),
        #[cfg(feature = "serde1")]
        (
            "serde_map",
            &kv::Value::from_serde(&MyValue {
                a: 1,
                b: "2",
                c: MyValue2 { d: 3.0 },
            }),
        ),
        #[cfg(feature = "serde1")]
        ("serde_array", &kv::Value::from_serde(&vec)),
        #[cfg(feature = "serde1")]
        ("serde_tuple", &kv::Value::from_serde(&(1u8, 2.0f64, "3"))),
    ];
    let kvs: &dyn kv::Source = &kvs;
    let record2 = Record::builder()
        .args(format_args!("arguments2 with \"quotes\""))
        .level(Level::Warn)
        .target("second_target")
        .module_path_static(Some("module_path1"))
        .file_static(Some("file2"))
        .line(Some(111))
        .key_values(kvs)
        .build();
    let record3 = Record::builder()
        .args(format_args!("panicking!"))
        .level(Level::Error)
        .target("panic")
        .build();

    #[cfg(feature = "serde1")]
    #[derive(serde::Serialize)]
    struct MyValue {
        a: usize,
        b: &'static str,
        c: MyValue2,
    }

    #[cfg(feature = "serde1")]
    #[derive(serde::Serialize)]
    struct MyValue2 {
        d: f64,
    }

    let tests = [
        (record1.clone(), true),
        (record1, false),
        (record2, true),
        (record3, true),
    ];

    for ((record, debug), want) in tests.into_iter().zip(expected) {
        let got = format_record::<F>(&record, debug);
        #[cfg(feature = "timestamp")]
        let want = add_timestamp(want.to_string(), SystemTime::now(), &got);
        assert_eq!(got, *want);
    }
    #[cfg(not(feature = "timestamp"))]
    let _ = add_timestamp;
}

fn format_record<F: Format>(record: &Record, debug: bool) -> String {
    let mut bufs = [IoSlice::new(&[]); BUFS_SIZE];
    let mut buf = format::Buffer::new();
    let bufs = F::format(&mut bufs, &mut buf, record, &NoKvs, debug);
    let mut output = Vec::new();
    let _ = output.write_vectored(bufs).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
#[cfg(feature = "timestamp")]
fn timestamp() {
    use std::mem::MaybeUninit;

    let tests = [
        SystemTime::now(),
        SystemTime::UNIX_EPOCH + Duration::from_secs(41 * (365 * 24 * 60 * 60)),
        SystemTime::UNIX_EPOCH + Duration::from_secs(51 * (365 * 24 * 60 * 60)),
        SystemTime::UNIX_EPOCH + Duration::from_secs(101 * (365 * 24 * 60 * 60)),
    ];

    for time in tests {
        // Get the libc values we expected.
        let diff = time
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

        let got = crate::timestamp::Timestamp::from(time);
        assert_eq!(got.year as i32, year);
        assert_eq!(got.month as i32, month);
        assert_eq!(got.day as i32, day);
        assert_eq!(got.hour as i32, hour);
        assert_eq!(got.min as i32, min);
        assert_eq!(got.sec as i32, sec);
        assert_eq!(got.micro, micros);
    }
}
