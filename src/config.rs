//! Configuration of the logger.

use std::env;
use std::marker::PhantomData;

use log::{LevelFilter, SetLoggerError};

use crate::format::{Format, Gcloud, LogFmt};
use crate::{Logger, Targets};

/// Configuration of the logger.
///
/// It support two logging formats:
///  * [`logfmt`](Config::logfmt) and
///  * [`gcloud`](Config::gcloud).
#[derive(Debug)]
pub struct Config<F> {
    filter: LevelFilter,
    targets: Targets,
    _format: PhantomData<F>,
}

impl Config<()> {
    /// Logfmt following <https://www.brandur.org/logfmt>.
    pub fn logfmt() -> Config<LogFmt> {
        Config::new()
    }

    /// Google Cloud Platform structured logging using JSON, following
    /// <https://cloud.google.com/logging/docs/structured-logging>.
    pub fn gcloud() -> Config<Gcloud> {
        Config::new()
    }
}

impl<F: Format + Send + Sync + 'static> Config<F> {
    fn new() -> Config<F> {
        Config {
            filter: get_max_level(),
            targets: get_log_targets(),
            _format: PhantomData,
        }
    }

    /// Initialise the logger.
    ///
    /// See the [crate level documentation] for more.
    ///
    /// [crate level documentation]: index.html
    ///
    /// # Panics
    ///
    /// This will panic if the logger fails to initialise. Use [`Config::try_init`] if
    /// you want to handle the error yourself.
    pub fn init(self) {
        self.try_init()
            .unwrap_or_else(|err| panic!("failed to initialise the logger: {}", err));
    }

    /// Try to initialise the logger.
    ///
    /// Unlike [`Config::init`] this doesn't panic when the logger fails to initialise.
    /// See the [crate level documentation] for more.
    ///
    /// [`init`]: fn.init.html
    /// [crate level documentation]: index.html
    pub fn try_init(self) -> Result<(), SetLoggerError> {
        let logger = Box::new(Logger {
            filter: self.filter,
            targets: self.targets,
            _format: self._format,
        });
        log::set_boxed_logger(logger)?;
        log::set_max_level(self.filter);

        #[cfg(all(feature = "log-panic", not(feature = "nightly")))]
        log_panics::init();
        #[cfg(all(feature = "log-panic", feature = "nightly"))]
        std::panic::set_hook(Box::new(log_panic));
        Ok(())
    }
}

/// Get the maximum log level based on the environment.
pub(crate) fn get_max_level() -> LevelFilter {
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
pub(crate) fn get_log_targets() -> Targets {
    match env::var("LOG_TARGET") {
        Ok(ref targets) if !targets.is_empty() => {
            Targets::Only(targets.split(',').map(|target| target.into()).collect())
        }
        _ => Targets::All,
    }
}

/// Panic hook that logs the panic using [`log::error!`].
#[cfg(all(feature = "log-panic", feature = "nightly"))]
fn log_panic(info: &std::panic::PanicInfo<'_>) {
    use std::backtrace::Backtrace;
    use std::thread;

    let thread = thread::current();
    let thread_name = thread.name().unwrap_or("unnamed");
    let msg = match info.payload().downcast_ref::<&'static str>() {
        Some(s) => *s,
        None => match info.payload().downcast_ref::<String>() {
            Some(s) => &**s,
            None => "<unknown>",
        },
    };
    let (file, line) = match info.location() {
        Some(location) => (location.file(), location.line()),
        None => ("<unknown>", 0),
    };
    let backtrace = Backtrace::force_capture();

    log::logger().log(
        &Record::builder()
            .args(format_args!(
                // NOTE: we include file in here because it's only logged when
                // debug severity is enabled.
                "thread '{}' panicked at '{}', {}:{}",
                thread_name, msg, file, line
            ))
            .level(log::Level::Error)
            .target("panic")
            .file(Some(file))
            .line(Some(line))
            .key_values(&("backtrace", &backtrace as &dyn std::fmt::Display))
            .build(),
    );
}
