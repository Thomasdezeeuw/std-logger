// Copyright 2017 Thomas de Zeeuw
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option. This file may not be
// used, copied, modified, or distributed except according to those terms.

// TODO: update doc below.
// TODO: update the description in Cargo.toml.
// TODO: doc the env variable for changing the log level.
// TODO: use the log crate to do actual logging.

// TODO: Add tests; reference some global buffer below used in testing.

//! A crate that holds a logging implementation that logs to standard error and
//! standard out. It uses standard error for all regular messages and standard
//! out for requests (when using the [`REQUEST_TARGET`]).
//!
//! [`REQUEST_TARGET`]: constant.REQUEST_TARGET.html

#![warn(missing_docs)]

extern crate log;
#[cfg(feature = "timestamp")]
extern crate chrono;
#[cfg(feature = "catch-panic")]
extern crate log_panics;

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
/// ```
///
/// For requests (using the [`REQUEST_TARGET`] target when logging, logged to
/// standard out):
///
/// ```text
/// timestamp [REQUEST]: message
/// ```
///
/// Note that the timestamp is not printed when the `timestamp` feature is not
/// enabled (this feature is enable by default).
///
/// If the `catch-panic` feature is enabled (enabled by default) this will also
/// catch and log any panics that occur.
///
/// [`REQUEST_TARGET`]: constant.REQUEST_TARGET.html
pub fn init() {
    log::set_logger(|max_level| {
        let filter = get_max_level();
        max_level.set(filter);
        Box::new(Logger { filter: filter })
    }).unwrap_or_else(|_| panic!("failed to initialize the logger"));

    #[cfg(feature = "catch-panic")]
    log_panics::init();
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
        LogLevelFilter::Warn
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
    let timestamp = chrono::Utc::now();
    match record.target() {
        REQUEST_TARGET => {
            write!(&mut stdout(), "{} [REQUEST]: {}\n",
                timestamp, record.args()
            ).unwrap_or_else(log_failure)
        },
        target => {
            write!(&mut stderr(), "{} [{}] {}: {}\n",
                timestamp, record.level(), target, record.args()
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
#[inline(always)]
fn stdout() -> Vec<u8> {
    Vec::new()
}

#[cfg(test)]
#[inline(always)]
fn stderr() -> Vec<u8> {
    Vec::new()
}
