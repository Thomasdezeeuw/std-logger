// Copyright 2017-2018 Thomas de Zeeuw
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option. This file may not be
// used, copied, modified, or distributed except according to those terms.

use std::{panic, str};
use std::default::Default;
use std::sync::Mutex;

use super::*;

lazy_static! {
    /// A global lock since most tests need to run in serial.
    static ref SERIAL_TEST_MUTEX: Mutex<()> = Mutex::new(());
}

/// Macro to crate a serial test, that locks the `SERIAL_TEST_MUTEX` while
/// testing.
macro_rules! serial_test {
    (fn $name:ident() $body:block) => {
        #[test]
        fn $name() {
            let guard = SERIAL_TEST_MUTEX.lock().unwrap();
            // Catch any panics to not poisen the lock.
            if let Err(err) = panic::catch_unwind(|| { $body }) {
                drop(guard);
                panic::resume_unwind(err);
            }
        }
    }
}

serial_test!{
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
}

serial_test!{
    fn log_output() {
        unsafe { log_setup(); }

        #[cfg(feature = "timestamp")]
        let timestamp = chrono::Utc::now();

        trace!("trace message");
        debug!("debug message");
        info!("info message");
        warn!("warn message");
        error!("error message");
        info!(target: REQUEST_TARGET, "request message");

        let want = vec![
            "[TRACE] std_logger::tests: trace message",
            "[DEBUG] std_logger::tests: debug message",
            "[INFO] std_logger::tests: info message",
            "[WARN] std_logger::tests: warn message",
            "[ERROR] std_logger::tests: error message",
            "[REQUEST]: request message",
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

/// This requires the `SERIAL_TEST_MUTEX` to be held!
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
