// Copyright 2017 Thomas de Zeeuw
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option. This file may not be
// used, copied, modified, or distributed except according to those terms.

//! A crate that holds a logging implementation that logs to standard error and
//! standard out. It uses standard error for all regular messages and standard
//! out for requests (when using the [`REQUEST_TARGET`]).
//!
//! # Severity
//!
//! You can use various envorinment variables to change the severity (log level)
//! of the logs to actually log and which to ignore.
//!
//! The `TRACE` variable sets the severity to the trace, meaning it will log
//! everything. `DEBUG` will set it to debug, one level higher then trace and it
//! will not log anything with a trace severity. `LOG` and `LOG_LEVEL` can be
//! used to set the severity to a specific value, see the [`log`]'s package
//! `LogLevelFilter` enum for available values. If none of these envorinment
//! variables are found it will default to an information severity.
//!
//! # Logging requests
//!
//! To log requests a special target is provided, [`REQUEST_TARGET`], this will
//! log these message to standard out rather then standard out. This allows for
//! seperate processing of error messages and requests. See the
//! [`REQUEST_TARGET`] constant for an example.
//!
//! # Crate features
//!
//! This crate has two features, both of which are enabled by default,
//! "timestamp" and "log-panic".
//!
//! ## Timestamp feature
//!
//! The "timestamp" feature adds a timestamp infront of every message. It uses
//! the format defined in [`RFC3339`] with 6 digit nanosecond precision, e.g.
//! `2017-08-21T13:50:53.383553Z`. This means that the timestamp is **always**
//! logged in UTC.
//!
//!
//! ## Log-panic feature
//!
//! The "log-panic" feature will log all panics using the `error` severity,
//! rather then using the default panic handler. It will log the panic message
//! as well as the location and a backtrace, see the log output below for an
//! example.
//!
//! ```log
//! [ERROR] panic: thread 'main' panicked at 'oops': examples/panic.rs:24
//! stack backtrace:
//!    0:        0x106ba8f74 - backtrace::backtrace::trace<closure>
//!                         at backtrace-0.3.2/src/backtrace/mod.rs:42
//!    1:        0x106ba49af - backtrace::capture::Backtrace::new::h54d7cfa8f40c5b43
//!                         at backtrace-0.3.2/src/capture.rs:64
//!    2:        0x106b9f4e6 - log_panics::init::{{closure}}
//!                         at log-panics-1.2.0/src/lib.rs:52
//!    3:        0x106bc6951 - std::panicking::rust_panic_with_hook::h6c19f9ba35264287
//!                         at src/libstd/panicking.rs:612
//!    4:        0x106b93146 - std::panicking::begin_panic<&str>
//!                         at src/libstd/panicking.rs:572
//!    5:        0x106b93bf1 - panic::main
//!                         at examples/panic.rs:24
//!    6:        0x106bc751c - __rust_maybe_catch_panic
//!                         at src/libpanic_unwind/lib.rs:98
//!    7:        0x106bc6c08 - std::rt::lang_start::h6f338c4ae2d58bbe
//!                         at src/libstd/rt.rs:61
//!    8:        0x106b93c29 - main
//! ```
//!
//! If the "timestamp" feature is enable the message will be prefixed with a
//! timestamp as described in the [Timestamp feature].
//!
//! # Note
//!
//! This crate provides only a logging implementation. To do actual logging use
//! the [`log`] crate and it's various macros.
//!
//! [`REQUEST_TARGET`]: constant.REQUEST_TARGET.html
//! [`log`]: https://crates.io/crates/log
//! [`RFC3339`]: https://tools.ietf.org/html/rfc3339
//! [Timestamp feature]: #timestamp-feature

#![warn(missing_docs)]

#[macro_use]
extern crate log;
#[cfg(feature = "timestamp")]
extern crate chrono;
#[cfg(feature = "log-panic")]
extern crate log_panics;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
mod tests;

use std::env;
use std::io::{self, Write};

use log::{Log, LogLevelFilter, LogMetadata, LogRecord};

/// The log target to use when logging requests. Using this as a target the
/// message will be logged to standard out.
///
/// ```
/// #[macro_use]
/// extern crate log;
/// extern crate std_logger;
///
/// use std_logger::REQUEST_TARGET;
///
/// # fn main() {
/// # let url = "/";
/// # let status = 200;
/// # let response_time = "20 ms";
/// // In for example a HTTP handler.
/// info!(target: REQUEST_TARGET, "url = {}, status = {}, response_time = {}",
///     url, status, response_time);
/// # }
/// ```
pub const REQUEST_TARGET: &'static str = "request";

/// Initialize the logger. Any logs with the target set to [`REQUEST_TARGET`]
/// will be logged to standard out, any other logs will be printed to standard
/// error. If the initializion fails this function will panic.
///
/// Logs are formatted using the following format. For messages (logged to
/// standard error):
///
/// ```text
/// timestamp [LOG_LEVEL] target: message
///
/// 2017-08-04T12:56:48.187155+00:00 [ERROR] my_module: my error message
/// ```
///
/// For requests (using the [`REQUEST_TARGET`] target when logging, logged to
/// standard out):
///
/// ```text
/// timestamp [REQUEST]: message
///
/// 2017-08-04T12:56:48.187182+00:00 [REQUEST]: my request message
/// ```
///
/// Note that the timestamp is not printed when the `timestamp` feature is not
/// enabled (this feature is enable by default).
///
/// If the `log-panic` feature is enabled (enabled by default) this will also
/// log any panics that occur.
///
/// [`REQUEST_TARGET`]: constant.REQUEST_TARGET.html
pub fn init() {
    let filter = get_max_level();
    log::set_logger(|max_level| {
        max_level.set(filter);
        Box::new(Logger { filter: filter })
    }).unwrap_or_else(|_| panic!("failed to initialize the logger"));

    #[cfg(feature = "log-panic")]
    log_panics::init();

    #[cfg(feature = "log-panic")]
    debug!("enabled std-logger with log level: {}, with logging of panics", filter);
    #[cfg(not(feature = "log-panic"))]
    debug!("enabled std-logger with log level: {}, no logging of panics", filter);
}

/// Get the maximum log level based on the environment.
fn get_max_level() -> LogLevelFilter {
    let vars = ["LOG", "LOG_LEVEL"];
    for var in &vars {
        if let Ok(level) = env::var(var) {
            if let Ok(level) = level.parse() {
                return level;
            }
        }
    }

    if env::var("TRACE").is_ok() {
        LogLevelFilter::Trace
    } else if env::var("DEBUG").is_ok() {
        LogLevelFilter::Debug
    } else {
        LogLevelFilter::Info
    }
}

/// A simple wrapper to implement `Log` on.
struct Logger {
    /// The filter used to determine what messages to log.
    filter: LogLevelFilter,
}

impl Log for Logger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        self.filter >= metadata.level()
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            log(record);
        }
    }
}

/// The actual logging of a record, including a timestamp. This should be kept
/// in sync with the same named function below.
#[cfg(feature = "timestamp")]
fn log(record: &LogRecord) {
    use chrono::{Datelike, Timelike};
    let timestamp = chrono::Utc::now();
    match record.target() {
        REQUEST_TARGET => {
            write!(&mut stdout(), "{:004}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z [REQUEST]: {}\n",
                timestamp.year(), timestamp.month(), timestamp.day(),
                timestamp.hour(), timestamp.minute(), timestamp.second(),
                timestamp.nanosecond() / 1000, record.args()
            ).unwrap_or_else(log_failure)
        },
        target => {
            write!(&mut stderr(), "{:004}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z [{}] {}: {}\n",
                timestamp.year(), timestamp.month(), timestamp.day(),
                timestamp.hour(), timestamp.minute(), timestamp.second(),
                timestamp.nanosecond() / 1000, record.level(), target, record.args()
            ).unwrap_or_else(log_failure)
        },
    }
}

/// The actual logging of a record, without a timestamp. This should be kept in
/// sync with the same named function above.
#[cfg(not(feature = "timestamp"))]
fn log(record: &LogRecord) {
    match record.target() {
        REQUEST_TARGET => {
            write!(&mut stdout(), "[REQUEST]: {}\n", record.args())
                .unwrap_or_else(log_failure)
        },
        target => {
            write!(&mut stderr(), "[{}] {}: {}\n",
                record.level(), target, record.args()
            ).unwrap_or_else(log_failure)
        },
    }
}

/// The function that gets called when we're unable to log a message.
#[inline(never)]
#[cold]
fn log_failure(err: io::Error) {
    panic!("unexpected error logging message: {}", err)
}

// Functions to get standard out/error, which are stubbed in testing. Even
// though the return type of the functions are different we only need them both
// to implement `io::Write`.

#[cfg(not(test))]
#[inline(always)]
fn stdout() -> io::Stdout {
    io::stdout()
}

#[cfg(not(test))]
#[inline(always)]
fn stderr() -> io::Stderr {
    io::stderr()
}

// The testing variant of the functions.

#[cfg(test)]
mod test_instruments {
    use std::io::{self, Write};
    use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

    // TODO: replace `LOG_OUTPUT` with type `[Option<Vec<u8>>; 10]`, once the
    // `drop_types_in_const` feature is stable, that would make all of this a
    // bit safer.

    /// The output of the log macros, *if this is not null it must point to
    /// valid memory*.
    pub static mut LOG_OUTPUT: *mut [Option<Vec<u8>>; 10] = 0 as *mut [Option<Vec<u8>>; 10];

    /// Maximum number of logs we can hold, keep in sync with above.
    const LOG_OUTPUT_MAX: usize = 10;

    /// Increase to get a position in the `LOG_OUTPUT` array.
    pub static LOG_OUTPUT_INDEX: AtomicUsize = ATOMIC_USIZE_INIT;

    /// Simple wrapper around a `Vec<u8>` which add itself to `LOG_OUTPUT` when
    /// dropped.
    pub struct LogOutput {
        /// Must always be something, until it's dropped.
        inner: Option<Vec<u8>>,
    }

    impl Write for LogOutput {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.inner.as_mut().unwrap().write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.inner.as_mut().unwrap().flush()
        }
    }

    impl Drop for LogOutput {
        fn drop(&mut self) {
            let output = self.inner.take().unwrap();
            let index = LOG_OUTPUT_INDEX.fetch_add(1, Ordering::SeqCst);
            if index >= LOG_OUTPUT_MAX {
                panic!("too many logs written, increase the size of `LOG_OUTPUT`");
            }
            unsafe {
                if let Some(log_output) = LOG_OUTPUT.as_mut() {
                    log_output[index] = Some(output);
                } else {
                    panic!("LOG_OUTPUT is not set, this is required in testing");
                }
            }
        }
    }

    #[inline(always)]
    pub fn stdout() -> LogOutput {
        LogOutput { inner: Some(Vec::new()) }
    }

    #[inline(always)]
    pub fn stderr() -> LogOutput {
        LogOutput { inner: Some(Vec::new()) }
    }
}

#[cfg(test)]
use test_instruments::{stdout, stderr, LOG_OUTPUT, LOG_OUTPUT_INDEX};
