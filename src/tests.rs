use std::default::Default;
use std::sync::Mutex;
use std::{env, panic, str};

use lazy_static::lazy_static;
use log::{debug, error, info, trace, warn, LevelFilter};

use crate::{
    get_log_targets, get_max_level, init, request, Targets, LOG_OUTPUT, LOG_OUTPUT_INDEX,
    REQUEST_TARGET,
};

/// Macro to create a group of sequential tests.
macro_rules! sequential_tests {
    ($(fn $name:ident() $body:block)+) => {
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
        let tests = vec![
            ("LOG", "TRACE", LevelFilter::Trace),
            ("LOG", "ERROR", LevelFilter::Error),
            ("LOG_LEVEL", "ERROR", LevelFilter::Error),
            ("LOG_LEVEL", "DEBUG", LevelFilter::Debug),
            ("TRACE", "1", LevelFilter::Trace),
            ("DEBUG", "1", LevelFilter::Debug),
        ];

        for test in tests {
            env::set_var(test.0, test.1);

            let want = test.2;
            let got = get_max_level();
            assert_eq!(want, got);

            env::remove_var(test.0);
        }
    }

    fn should_get_correct_log_targets() {
        let tests = vec![
            ("", Targets::All),
            ("crate1", Targets::Only(vec!["crate1".into()].into_boxed_slice())),
            ("crate1::mod1", Targets::Only(vec!["crate1::mod1".into()].into_boxed_slice())),
            ("crate1,crate2", Targets::Only(vec!["crate1".into(), "crate2".into()].into_boxed_slice())),
        ];

        for test in tests {
            env::set_var("LOG_TARGET", test.0);

            let want = test.1;
            let got = get_log_targets();
            assert_eq!(want, got);
        }

        env::remove_var("LOG_TARGET");
        assert_eq!(get_log_targets(), Targets::All);
    }

    fn log_output() {
        unsafe { log_setup(); }

        #[cfg(feature = "timestamp")]
        let timestamp = chrono::Utc::now();

        trace!("trace message");
        debug!("debug message");
        info!("info message");
        warn!("warn message");
        error!("error message");
        info!(target: REQUEST_TARGET, "request message1");
        request!("request message2");

        let want = vec![
            "[TRACE] std_logger::tests (src/tests.rs:83): trace message",
            "[DEBUG] std_logger::tests (src/tests.rs:84): debug message",
            "[INFO] std_logger::tests (src/tests.rs:85): info message",
            "[WARN] std_logger::tests (src/tests.rs:86): warn message",
            "[ERROR] std_logger::tests (src/tests.rs:87): error message",
            "[REQUEST] std_logger::tests (src/tests.rs:88): request message1",
            "[REQUEST] std_logger::tests (src/tests.rs:89): request message2",
        ];
        let mut got = unsafe {
            (&*LOG_OUTPUT).iter()
        };

        let mut got_length = 0;
        let mut want_iter = want.iter();
        loop {
            match (want_iter.next(), got.next()) {
                (Some(want), Some(got)) if got.is_some() => {
                    let got = got.as_ref().unwrap();
                    let got = str::from_utf8(got).expect("unable to parse string").trim();

                    #[allow(unused_mut)]
                    let mut want = (*want).to_owned();
                    #[cfg(feature = "timestamp")]
                    { want = add_timestamp(want, timestamp, got); }

                    // TODO: for some reason this failure doesn't shows itself in the
                    // output, hence this workaround.
                    println!("Comparing:");
                    println!("want: {}", want);
                    println!("got:  {}", got);
                    assert_eq!(got, want.as_str(), "message differ");

                    got_length += 1;
                },
                _ => break,
            }
        }

        if got_length != want.len() {
            panic!("the number of log messages got differs from the amount of messages wanted");
        }
    }
}

/// This requires the `SEQUENTIAL_TEST_MUTEX` to be held!
unsafe fn log_setup() {
    use std::sync::atomic::Ordering;

    if !LOG_OUTPUT.is_null() {
        for output in (&mut *LOG_OUTPUT).iter_mut().skip(1) {
            drop(output.take());
        }
        LOG_OUTPUT_INDEX.store(1, Ordering::Relaxed);
        return;
    }

    let output = Box::new(Default::default());
    LOG_OUTPUT = Box::into_raw(output);

    env::set_var("LOG_LEVEL", "TRACE");
    init();
    env::remove_var("LOG_LEVEL");
}

#[cfg(feature = "timestamp")]
fn add_timestamp(message: String, timestamp: chrono::DateTime<chrono::Utc>, got: &str) -> String {
    use chrono::{Datelike, Timelike};

    // Add the timestamp to the expected string.
    let timestamp = format!(
        "{:004}-{:02}-{:02}T{:02}:{:02}:{:02}.{}Z",
        timestamp.year(),
        timestamp.month(),
        timestamp.day(),
        timestamp.hour(),
        timestamp.minute(),
        timestamp.second(),
        &got[20..26]
    );
    format!("{} {}", timestamp, message)
}

#[test]
fn targets_should_log() {
    let targets = vec![
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
