//! A crate that holds a logging implementation that logs to standard error and
//! standard out. It uses standard error for all regular messages and standard
//! out for requests.
//!
//! This crate provides only a logging implementation. To do actual logging use
//! the [`log`] crate and it's various macros.
//!
//!
//! # Setting severity
//!
//! You can use various environment variables to change the severity (log level)
//! of the messages to actually log and which to ignore.
//!
//! `LOG` and `LOG_LEVEL` can be used to set the severity to a specific value,
//! see the [`log`]'s package `LevelFilter` type for available values.
//!
//! ```bash
//! # In your shell of choose:
//!
//! # Set the log severity to only print log message with info severity or
//! # higher, trace and debug messages won't be printed anymore.
//! $ LOG=info ./my_binary
//!
//! # Set the log severity to only print log message with warning severity or
//! # higher, informational (or lower severity) messages won't be printed
//! # anymore.
//! $ LOG=warn ./my_binary
//! ```
//!
//! Alternatively setting the `TRACE` variable (e.g. `TRACE=1`) sets the
//! severity to the trace, meaning it will log everything. Setting `DEBUG` will
//! set the severity to debug.
//!
//! ```bash
//! # In your shell of choose:
//!
//! # Enables trace logging.
//! $ TRACE=1 ./my_binary
//!
//! # Enables debug logging.
//! $ DEBUG=1 ./my_binary
//! ```
//!
//! If none of these environment variables are found it will default to an
//! information severity.
//!
//!
//! # Logging requests
//!
//! To log requests a special target is provided: [`REQUEST_TARGET`]. This will
//! cause the message to be logged to standard out, rather then standard error.
//! This allows for separate processing of error messages and request logs.
//!
//! ```
//! #[macro_use]
//! extern crate log;
//! extern crate std_logger;
//!
//! use std_logger::REQUEST_TARGET;
//!
//! # fn main() {
//! info!(target: REQUEST_TARGET, "Got a request!");
//! # }
//! ```
//!
//!
//! # Limiting logging targets
//!
//! Sometimes it's useful to only log messages related to a specific target, for
//! example when debugging a single function you might want only see messages
//! from the module the function is in. This can be achieved by using the
//! `LOG_TARGET` environment variable.
//!
//! ```bash
//! # In your shell of choose:
//!
//! # Only log messages from your crate.
//! $ LOG_TARGET=my_crate ./my_binary
//!
//! # Only log messages from the `my_module` module in your crate.
//! $ LOG_TARGET=my_crate::my_module ./my_binary
//!
//! # Multiple log targets are also supported by separating the values by a
//! # comma.
//! $ LOG_TARGET=my_crate::my_module,my_crate::my_other_module ./my_binary
//!
//! # Very useful in combination with trace severity to get all messages you
//! # want, but filter out the message you don't need.
//! $ LOG_LEVEL=trace LOG_TARGET=my_crate::my_module ./my_binary
//! ```
//!
//! Note that [requests] are always logged.
//!
//! [requests]: index.html#logging-requests
//!
//!
//! # Format
//!
//! For regular messages, printed to standard error, the following format is
//! used:
//!
//! ```text
//! timestamp [LOG_LEVEL] target: message
//!
//! For example:
//!
//! 2018-03-24T13:48:28.820588Z [ERROR] my_module: my error message
//! ```
//!
//! For requests, logged using the [`REQUEST_TARGET`] target and printed to
//! standard out, the following format is used:
//!
//! ```text
//! timestamp [REQUEST]: message
//!
//! For example:
//!
//! 2018-03-24T13:30:28.820588Z [REQUEST]: my request message
//! ```
//!
//! Note: the timestamp is not printed when the *timestamp* feature is not
//! enabled, this feature is enabled by default, see [Timestamp feature] below.
//!
//!
//! # Crate features
//!
//! This crate has two features, both of which are enabled by default,
//! *timestamp* and *log-panic*.
//!
//!
//! ## Timestamp feature
//!
//! The *timestamp* feature adds a timestamp in front of every message. It uses
//! the format defined in [`RFC3339`] with 6 digit nanosecond precision, e.g.
//! `2018-03-24T13:48:48.063934Z`. This means that the timestamp is **always**
//! logged in UTC.
//!
//!
//! ## Log-panic feature
//!
//! The *log-panic* feature will log all panics using the `error` severity,
//! rather then using the default panic handler. It will log the panic message
//! as well as the location and a backtrace, see the log output below for an
//! example (this example doesn't include a timestamp).
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
//! If the *timestamp* feature is enable the message will be prefixed with a
//! timestamp as described in the [Timestamp feature].
//!
//!
//! # Example
//!
//! ```
//! #[macro_use]
//! extern crate log;
//! extern crate std_logger;
//!
//! # use std::time::Duration;
//! #
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
//! /// This our example request handler, just pretend it gets called with a
//! /// request.
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

#![warn(anonymous_parameters,
        bare_trait_objects,
        missing_debug_implementations,
        missing_docs,
        trivial_casts,
        trivial_numeric_casts,
        unused_extern_crates,
        unused_import_braces,
        unused_qualifications,
        unused_results,
        variant_size_differences,
)]

use std::cell::RefCell;
use std::env;
use std::io::{self, Write};

use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};

#[cfg(feature = "timestamp")]
use chrono::{Datelike, Timelike};

#[cfg(test)]
mod tests;

/// Target for logging requests.
///
/// See the [crate level documentation] for more.
///
/// [crate level documentation]: index.html#logging-requests
pub const REQUEST_TARGET: &str = "request";

/// Initialise the logger.
///
/// See the [crate level documentation] for more.
///
/// [crate level documentation]: index.html
///
/// # Panics
///
/// This will panic if the logger fails to initialise.
pub fn init() {
    try_init()
        .unwrap_or_else(|err| panic!("failed to initialise the logger: {}", err));
}

/// Try to initialise the logger.
///
/// Unlike [`init`] this doesn't panic when the logger fails to initialise. See
/// the [crate level documentation] for more.
///
/// [`init`]: fn.init.html
/// [crate level documentation]: index.html
pub fn try_init() -> Result<(), SetLoggerError> {
    let filter = get_max_level();
    let targets = get_log_targets();
    let logger = Logger { filter, targets };
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(filter);

    #[cfg(feature = "log-panic")]
    log_panics::init();
    Ok(())
}

/// Get the maximum log level based on the environment.
fn get_max_level() -> LevelFilter {
    for var in &["LOG", "LOG_LEVEL"] {
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

/// Get the targets to log, if any.
fn get_log_targets() -> Targets {
    match env::var("LOG_TARGET") {
        Ok(ref targets) if !targets.is_empty() =>
            Targets::Only(targets.split(',')
                .map(|target| target.to_owned())
                .collect()),
        _ => Targets::All,
    }
}

/// Our `Log` implementation.
struct Logger {
    /// The filter used to determine what messages to log.
    filter: LevelFilter,
    /// What logging targets to log.
    targets: Targets,
}

#[derive(Debug, Eq, PartialEq)]
enum Targets {
    /// Log all targets.
    All,
    /// Only log certain targets.
    Only(Vec<String>),
}

impl Targets {
    /// Returns true if all targets should be logged, or if the target of the
    /// `metadata` should be logger.
    fn should_log(&self, target: &str) -> bool {
        if target == REQUEST_TARGET {
            // Always log requests.
            true
        } else if let Targets::Only(targets) = self {
            // Log all targets that start with an allowed target. This way we
            // can just use `LOG_TARGET=my_crate`, rather then
            // `LOG_TARGET=my_crate::module1,my_crate::module2` etc.
            targets.iter().any(|log_target| target.starts_with(log_target))
        } else {
            // All targets should be logged.
            true
        }
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.filter >= metadata.level()
            && self.targets.should_log(metadata.target())
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            log(record);
        }
    }

    fn flush(&self) {}
}

/// The actual logging of a record.
fn log(record: &Record) {
    // Thread local buffer for logging. This way we only lock standard out/error
    // for a single write call and don't create half written logs.
    thread_local! {
        static BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(1024));
    }

    #[cfg(feature = "timestamp")]
    let timestamp = chrono::Utc::now();

    BUFFER.with(|buffer| {
        let mut buffer = buffer.borrow_mut();
        buffer.clear();

        #[cfg(feature = "timestamp")]
        write!(
            &mut buffer,
            "{:004}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z ",
            timestamp.year(),
            timestamp.month(),
            timestamp.day(),
            timestamp.hour(),
            timestamp.minute(),
            timestamp.second(),
            timestamp.nanosecond() / 1000,
        ).unwrap_or_else(log_failure);

        match record.target() {
            REQUEST_TARGET => {
                writeln!(&mut buffer, "[REQUEST]: {}", record.args())
                    .unwrap_or_else(log_failure);

                write_once(stdout(), &buffer).unwrap_or_else(log_failure);
            },
            target => {
                writeln!(&mut buffer, "[{}] {}: {}", record.level(), target, record.args())
                    .unwrap_or_else(log_failure);

                write_once(stderr(), &buffer).unwrap_or_else(log_failure);
            },
        }
    });
}

/// Write the entire `buf`fer into the `output` or return an error.
#[inline(always)]
fn write_once<W>(mut output: W, buf: &[u8]) -> io::Result<()>
    where W: Write,
{
    output.write(buf).and_then(|written| if written != buf.len() {
        // Not completely correct when going by the name alone, but it's the
        // closest we can get to a descriptive error.
        Err(io::ErrorKind::WriteZero.into())
    } else {
        Ok(())
    })
}

/// The function that gets called when we're unable to print a message.
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
    use std::ptr::null_mut;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // TODO: replace `LOG_OUTPUT` with type `[Option<Vec<u8>>; 10]`, once the
    // `drop_types_in_const` feature is stable, that would make all of this a
    // bit safer.

    /// Maximum number of logs we can hold.
    const LOG_OUTPUT_MAX: usize = 10;

    /// The output of the log macros, **if this is not null it must point to
    /// valid memory**.
    pub static mut LOG_OUTPUT: *mut [Option<Vec<u8>>; LOG_OUTPUT_MAX] = null_mut();

    /// Increase to get a position in the `LOG_OUTPUT` array.
    pub static LOG_OUTPUT_INDEX: AtomicUsize = AtomicUsize::new(0);

    /// Simple wrapper around a `Vec<u8>` which adds itself to `LOG_OUTPUT` when
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
use self::test_instruments::{stderr, stdout, LOG_OUTPUT, LOG_OUTPUT_INDEX};
