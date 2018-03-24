// Copyright 2017 Thomas de Zeeuw
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option. This file may not be
// used, copied, modified, or distributed except according to those terms.

//! A crate that holds a logging implementation that logs to standard error and
//! standard out. It uses standard error for all regular messages and standard
//! out for requests.
//!
//!
//! # Severity
//!
//! You can use various envorinment variables to change the severity (log level)
//! of the messages to actually log and which to ignore.
//!
//! Setting the `TRACE` variable (e.g. `TRACE=1`) sets the severity to the
//! trace, meaning it will log everything. Setting `DEBUG` will set the severity
//! to debug, one level higher then trace and it will not log anything with a
//! trace severity. `LOG` and `LOG_LEVEL` can be used to set the severity to a
//! specific value, see the [`log`]'s package `LevelFilter` enum for available
//! values. If none of these envorinment variables are found it will default to
//! an information severity.
//!
//!
//! # Logging requests
//!
//! To log requests a special target is provided, [`REQUEST_TARGET`], this will
//! log these message to standard out rather then standard out. This allows for
//! seperate processing of error messages and requests. See the
//! [`REQUEST_TARGET`] constant for an example.
//!
//!
//! # Format
//!
//! Logs are formatted using the following format. For messages (logged to
//! standard error):
//!
//! ```text
//! timestamp [LOG_LEVEL] target: message
//! ```
//!
//! For example:
//!
//! ```text
//! 2017-08-04T12:56:48.187155+00:00 [ERROR] my_module: my error message
//! ```
//!
//! For requests (using the [`REQUEST_TARGET`] target when logging, logged to
//! standard out):
//!
//! ```text
//! timestamp [REQUEST]: message
//! ```
//!
//! For example:
//!
//! ```text
//! 2017-08-04T12:56:48.187182+00:00 [REQUEST]: my request message
//! ```
//!
//! Note: the timestamp is not printed when the "timestamp" feature is not
//! enabled (this feature is enabled by default), see [Timestamp feature].
//!
//!
//! # Crate features
//!
//! This crate has two features, both of which are enabled by default,
//! "timestamp" and "log-panic".
//!
//!
//! ## Timestamp feature
//!
//! The "timestamp" feature adds a timestamp in front of every message. It uses
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
//!
//! # Note
//!
//! This crate provides only a logging implementation. To do actual logging use
//! the [`log`] crate and it's various macros.
//!
//!
//! # Example
//!
//! ```
//! #[macro_use]
//! extern crate log;
//! extern crate std_logger;
//!
//! use std_logger::REQUEST_TARGET;
//!
//! fn main() {
//!     // First thing we need to do is initialise the logger before anything
//!     // else.
//!     std_logger::init();
//!
//!     // Now we can start logging!
//!     info!("Our application started!");
//!
//!     // Do useful stuff, like starting a HTTP server
//! }
//!
//! # struct Request {
//! #   url: String,
//! #   status: u16,
//! #   response_time: Duration,
//! # }
//! #
//! fn log_handler(req: Request) {
//!     // This will be logged to standard out, rather then standard error.
//!     info!(target: REQUEST_TARGET, "url = {}, status = {}, response_time = {:?}",
//!         req.url, req.status, req.response_time);
//! }
//! ```
//!
//! [`REQUEST_TARGET`]: constant.REQUEST_TARGET.html
//! [`log`]: https://crates.io/crates/log
//! [`RFC3339`]: https://tools.ietf.org/html/rfc3339
//! [Timestamp feature]: #timestamp-feature

#![warn(missing_debug_implementations,
        missing_docs,
        trivial_casts,
        trivial_numeric_casts,
        unused_import_braces,
        unused_qualifications,
        unused_results,
)]

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

use log::{LevelFilter, Log, Metadata, Record};

/// Target for logging requests.
///
/// See the [crate level documentation] for more.
///
/// [crate level documentation]: index.html
pub const REQUEST_TARGET: &'static str = "request";

/// Initialise the logger.
///
/// See the [crate level documentation] for more.
///
/// [crate level documentation]: index.html
pub fn init() {
    let filter = get_max_level();
    let logger = Logger { filter };
    log::set_boxed_logger(Box::new(logger))
        .unwrap_or_else(|_| panic!("failed to initialize the logger"));
    log::set_max_level(filter);

    #[cfg(feature = "log-panic")]
    log_panics::init();
}

/// Get the maximum log level based on the environment.
fn get_max_level() -> LevelFilter {
    const VARS: [&'static str; 2] = ["LOG", "LOG_LEVEL"];
    for var in &VARS {
        if let Ok(level) = env::var(var) {
            if let Ok(level) = level.parse() {
                return level;
            }
        }
    }

    if env::var("TRACE").is_ok() {
        LevelFilter::Trace
    } else if env::var("DEBUG").is_ok() {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    }
}

/// A simple struct which implements `Log`.
struct Logger {
    /// The filter used to determine what messages to log.
    filter: LevelFilter,
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.filter >= metadata.level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            log(record);
        }
    }

    fn flush(&self) { }
}

/// The actual logging of a record, including a timestamp. This should be kept
/// in sync with the same named function below.
#[cfg(feature = "timestamp")]
fn log(record: &Record) {
    use chrono::{Datelike, Timelike};
    let timestamp = chrono::Utc::now();
    match record.target() {
        REQUEST_TARGET => write!(
            &mut stdout(),
            "{:004}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z [REQUEST]: {}\n",
            timestamp.year(),
            timestamp.month(),
            timestamp.day(),
            timestamp.hour(),
            timestamp.minute(),
            timestamp.second(),
            timestamp.nanosecond() / 1000,
            record.args()
        ).unwrap_or_else(log_failure),
        target => write!(
            &mut stderr(),
            "{:004}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z [{}] {}: {}\n",
            timestamp.year(),
            timestamp.month(),
            timestamp.day(),
            timestamp.hour(),
            timestamp.minute(),
            timestamp.second(),
            timestamp.nanosecond() / 1000,
            record.level(),
            target,
            record.args()
        ).unwrap_or_else(log_failure),
    }
}

/// The actual logging of a record, without a timestamp. This should be kept in
/// sync with the same named function above.
#[cfg(not(feature = "timestamp"))]
fn log(record: &Record) {
    match record.target() {
        REQUEST_TARGET => write!(&mut stdout(), "[REQUEST]: {}\n", record.args())
            .unwrap_or_else(log_failure),
        target => write!(&mut stderr(), "[{}] {}: {}\n", record.level(), target, record.args())
            .unwrap_or_else(log_failure),
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
    use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

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
use test_instruments::{stderr, stdout, LOG_OUTPUT, LOG_OUTPUT_INDEX};
