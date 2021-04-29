use std::io::{IoSlice, Write};
use std::mem::replace;
use std::ops::Deref;
use std::sync::Mutex;
#[cfg(feature = "timestamp")]
use std::time::{Duration, SystemTime};
use std::{env, fmt, panic, str};

use lazy_static::lazy_static;
use log::{debug, error, info, kv, trace, warn, Level, LevelFilter, Record};

use crate::{
    format, get_log_targets, get_max_level, init, request, Targets, LOG_OUTPUT, REQUEST_TARGET,
};

/// Macro to create a group of sequential tests.
macro_rules! sequential_tests {
    ( $(fn $name: ident () $body: block)+ ) => {
        lazy_static! {
            /// A global lock for testing sequentially.
            static ref SEQUENTIAL_TEST_MUTEX: Mutex<()> = Mutex::new(());
        }

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
        init();
        env::remove_var("LOG_LEVEL");

        let want = &[
            "lvl=\"TRACE\" msg=\"trace message\" target=\"std_logger::tests\" module=\"std_logger::tests\" file=\"src/tests.rs:105\"\n",
            "lvl=\"DEBUG\" msg=\"debug message\" target=\"std_logger::tests\" module=\"std_logger::tests\" file=\"src/tests.rs:106\"\n",
            "lvl=\"INFO\" msg=\"info message\" target=\"std_logger::tests\" module=\"std_logger::tests\" file=\"src/tests.rs:107\"\n",
            "lvl=\"WARN\" msg=\"warn message\" target=\"std_logger::tests\" module=\"std_logger::tests\" file=\"src/tests.rs:108\"\n",
            "lvl=\"ERROR\" msg=\"error message\" target=\"std_logger::tests\" module=\"std_logger::tests\" file=\"src/tests.rs:109\"\n",
            "lvl=\"INFO\" msg=\"request message1\" target=\"request\" module=\"std_logger::tests\" file=\"src/tests.rs:110\"\n",
            "lvl=\"INFO\" msg=\"request message2\" target=\"request\" module=\"std_logger::tests\" file=\"src/tests.rs:111\"\n",
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

        let got = replace(&mut *(LOG_OUTPUT.lock().unwrap()), Vec::new());
        // Make sure the panics aren't logged.
        let _ = std::panic::take_hook();

        let mut got_length = 0;
        for (want, got) in want.into_iter().zip(got.into_iter()) {
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

#[cfg(feature = "timestamp")]
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
    let timestamp = format!(
        "ts=\"{:004}-{:02}-{:02}T{:02}:{:02}:{:02}.{}Z\"",
        year,
        month,
        day,
        hour,
        min,
        sec,
        &got[24..30] // We can never match the microseconds, so we just copy them.
    );
    format!("{} {}", timestamp, message)
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
        ("panic", vec![true, true, true, true]),
    ];

    for (test_target, wanted) in tests {
        for (target, want) in targets.iter().zip(wanted) {
            assert_eq!(
                target.should_log(test_target),
                want,
                "targets to log: {:?}, logging target: {}",
                target,
                test_target
            )
        }
    }
}

#[test]
fn format() {
    struct MyDisplay;

    impl fmt::Display for MyDisplay {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "MyDisplay")
        }
    }

    let record1 = Record::builder()
        .args(format_args!("some arguments1"))
        .level(Level::Info)
        .target("some_target1")
        .module_path_static(Some("module_path1"))
        .file_static(Some("file1"))
        .line(Some(123))
        .key_values(&("key1", "value1"))
        .build();
    let kvs = &[
        ("key2a", (&"value2") as &dyn kv::ToValue),
        ("key2b", &123u64),
        ("key3c", &-123i64),
        ("key3d", &123.0f64),
        ("key2e", &true),
        ("key2f", &false),
        ("key2g", &'c'),
        ("key2g", &(&MyDisplay as &dyn fmt::Display)),
    ];
    let kvs: &[(&str, &dyn kv::ToValue)] = kvs.deref();
    let kvs: &dyn kv::Source = &kvs;
    let record2 = Record::builder()
        .args(format_args!("arguments2"))
        .level(Level::Error)
        .target("second_target")
        .module_path_static(Some("module_path1"))
        .file_static(Some("file2"))
        .line(Some(111))
        .key_values(kvs)
        .build();

    let tests = &[
        (record1.clone(), true, "lvl=\"INFO\" msg=\"some arguments1\" target=\"some_target1\" module=\"module_path1\" key1=\"value1\" file=\"file1:123\"\n"),
        (record1, false, "lvl=\"INFO\" msg=\"some arguments1\" target=\"some_target1\" module=\"module_path1\" key1=\"value1\"\n"),
        (record2, true, "lvl=\"ERROR\" msg=\"arguments2\" target=\"second_target\" module=\"module_path1\" key2a=\"value2\" key2b=123 key3c=-123 key3d=123.0 key2e=true key2f=false key2g=\"c\" key2g=\"MyDisplay\" file=\"file2:111\"\n"),
    ];

    for (record, debug, want) in tests {
        let got = format_record(record, *debug);
        #[allow(unused_mut)]
        let mut want = (*want).to_owned();
        #[cfg(feature = "timestamp")]
        {
            want = add_timestamp(want, SystemTime::now(), &got)
        }

        assert_eq!(got, *want);
    }
}

fn format_record(record: &Record, debug: bool) -> String {
    let mut bufs = [IoSlice::new(&[]); crate::BUFS_SIZE];
    let mut buf = format::Buffer::new();
    let bufs = format::record(&mut bufs, &mut buf, record, debug);
    let mut output = Vec::new();
    let _ = output.write_vectored(bufs).unwrap();
    String::from_utf8(output).unwrap()
}
