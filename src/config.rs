//! Configuration of the logger.

use std::env;
use std::marker::PhantomData;

use log::{kv, LevelFilter, SetLoggerError};

use crate::format::{Format, Gcloud, Json, LogFmt};
#[cfg(feature = "log-panic")]
use crate::PANIC_TARGET;
use crate::{Logger, Targets};

/// Configuration of the logger.
///
/// It support three logging formats:
///  * [`logfmt`](Config::logfmt) and
///  * [`json`](Config::json) and
///  * [`gcloud`](Config::gcloud).
#[derive(Debug)]
pub struct Config<F, Kvs> {
    filter: LevelFilter,
    add_loc: Option<bool>,
    targets: Targets,
    kvs: Kvs,
    _format: PhantomData<F>,
}

impl Config<(), NoKvs> {
    /// Logfmt following <https://www.brandur.org/logfmt>.
    pub fn logfmt() -> Config<LogFmt, NoKvs> {
        Config::new(NoKvs)
    }

    /// Structured logging using JSON.
    pub fn json() -> Config<Json, NoKvs> {
        Config::new(NoKvs)
    }

    /// Google Cloud Platform structured logging using JSON, following
    /// <https://cloud.google.com/logging/docs/structured-logging>.
    pub fn gcloud() -> Config<Gcloud, NoKvs> {
        Config::new(NoKvs)
    }
}

impl<F, Kvs> Config<F, Kvs>
where
    F: Format + Send + Sync + 'static,
    Kvs: kv::Source + Send + Sync + 'static,
{
    fn new(kvs: Kvs) -> Config<F, Kvs> {
        Config {
            filter: get_max_level(),
            add_loc: None,
            targets: get_log_targets(),
            kvs,
            _format: PhantomData,
        }
    }

    /// Add the key-values `kvs` to all logged messages.
    pub fn with_kvs<K>(self, kvs: K) -> Config<F, K>
    where
        K: kv::Source + Send + Sync + 'static,
    {
        Config {
            filter: self.filter,
            add_loc: self.add_loc,
            targets: self.targets,
            kvs,
            _format: self._format,
        }
    }

    /// Enable or disable logging of the call location.
    ///
    /// Default to enable if the debug (or lower) messages are enabled.
    pub fn with_call_location(self, enable: bool) -> Config<F, Kvs> {
        Config {
            filter: self.filter,
            add_loc: Some(enable),
            targets: self.targets,
            kvs: self.kvs,
            _format: self._format,
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
            .unwrap_or_else(|err| panic!("failed to initialise the logger: {err}"));
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
            add_loc: self.add_loc.unwrap_or(self.filter >= LevelFilter::Debug),
            targets: self.targets,
            kvs: self.kvs,
            _format: self._format,
        });
        log::set_boxed_logger(logger)?;
        log::set_max_level(self.filter);

        #[cfg(feature = "log-panic")]
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
            Targets::Only(targets.split(',').map(Into::into).collect())
        }
        _ => Targets::All,
    }
}

/// Panic hook that logs the panic using [`log::error!`].
#[cfg(feature = "log-panic")]
fn log_panic(info: &std::panic::PanicInfo<'_>) {
    use std::backtrace::Backtrace;
    use std::thread;

    let mut record = log::Record::builder();
    let thread = thread::current();
    let thread_name = thread.name().unwrap_or("unnamed");
    let backtrace = Backtrace::force_capture();

    let key_values = [
        ("backtrace", kv::Value::capture_display(&backtrace)),
        ("thread_name", kv::Value::from(thread_name)),
    ];
    let key_values = key_values.as_slice();

    let _ = record
        .level(log::Level::Error)
        .target(PANIC_TARGET)
        .key_values(&key_values);

    if let Some(location) = info.location() {
        let _ = record
            .file(Some(location.file()))
            .line(Some(location.line()));
    };

    // Format for {info}: "panicked at '$message', $file:$line:$col".
    log::logger().log(
        &record
            .args(format_args!("thread '{thread_name}' {info}"))
            .build(),
    );
}

/// No initial key-values.
#[derive(Debug)]
pub struct NoKvs;

impl kv::Source for NoKvs {
    fn visit<'kvs>(&'kvs self, _: &mut dyn log::kv::Visitor<'kvs>) -> Result<(), log::kv::Error> {
        Ok(())
    }

    fn get<'v>(&'v self, _: log::kv::Key<'_>) -> Option<log::kv::Value<'v>> {
        None
    }

    fn count(&self) -> usize {
        0
    }
}
