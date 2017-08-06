// Copyright 2017 Thomas de Zeeuw
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option. This file may not be
// used, copied, modified, or distributed except according to those terms.

use std::default::Default;
use std::str;

use super::*;

#[test]
fn should_get_the_correct_log_level_from_env() {
    let tests = vec![
        ("LOG", "TRACE", LogLevelFilter::Trace),
        ("LOG", "ERROR", LogLevelFilter::Error),
        ("LOG_LEVEL", "ERROR", LogLevelFilter::Error),
        ("LOG_LEVEL", "DEBUG", LogLevelFilter::Debug),
        ("TRACE", "1", LogLevelFilter::Trace),
        ("DEBUG", "1", LogLevelFilter::Debug),
    ];

    for test in tests {
        env::set_var(test.0, test.1);

        let want = test.2;
        let got = get_max_level();
        assert_eq!(want, got);

        env::remove_var(test.0);
    }
}

#[test]
fn log_output() {
    let output = Box::new(Default::default());
    unsafe {
        LOG_OUTPUT = Box::into_raw(output);
    }

    env::set_var("LOG_LEVEL", "TRACE");
    init();
    env::remove_var("LOG_LEVEL");

    #[cfg(feature = "timestamp")]
    let timestamp = chrono::Utc::now();

    trace!("trace message");
    debug!("debug message");
    info!("info message");
    warn!("warn message");
    error!("error message");
    info!(target: REQUEST_TARGET, "request message");

    let want = vec![
        #[cfg(feature = "log-panic")]
        "[DEBUG] std_logger: enabled std-logger with log level: TRACE, with logging of panics",
        #[cfg(not(feature = "log-panic"))]
        "[DEBUG] std_logger: enabled std-logger with log level: TRACE, no logging of panics",
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

    #[cfg(feature = "log-panic")]
    {
        use std::panic;

        assert!(panic::catch_unwind(|| panic!("oops")).is_err());

        // Get the timetamp after causing the panic to (hopefully) reduce the
        // flakyness of this test.
        #[cfg(feature = "timestamp")]
        let timestamp = chrono::Utc::now();

        let output = unsafe { (&*LOG_OUTPUT)[got_length].as_ref() };
        if let Some(output) = output {
            use std::path::MAIN_SEPARATOR;
            let got = str::from_utf8(output).expect("unable to parse string").trim();
            let mut want = format!("[ERROR] panic: thread \'tests::log_output\' \
                panicked at \'oops\': src{}tests.rs:105", MAIN_SEPARATOR);
            #[cfg(feature = "timestamp")]
            { want = add_timestamp(want, timestamp, got); }

            println!("Comparing:");
            println!("want: {}", want);
            println!("got:  {}", &got[0..want.len()]);
            assert!(got.starts_with(&want));
        }
    }
}

#[cfg(feature = "timestamp")]
fn add_timestamp(message: String, timestamp: chrono::DateTime<chrono::Utc>, got: &str) -> String {
    use chrono::{Datelike, Timelike};

    // Add the timestamp to the expected string.
    let timestamp = format!("{:004}-{:02}-{:02}T{:02}:{:02}:{:02}.{}Z",
        timestamp.year(), timestamp.month(), timestamp.day(),
        timestamp.hour(), timestamp.minute(), timestamp.second(),
        &got[20..26]);
    format!("{} {}", timestamp, message)
}
