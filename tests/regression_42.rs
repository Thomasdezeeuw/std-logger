//! Regression test for issue #42.

#![cfg(feature = "log-panic")]

use std::fmt;

use log::info;

/// Panicking while logging triggers another `log!`.
#[test]
#[should_panic = "panic during formatting"]
fn panicking_while_logging() {
    std_logger::Config::logfmt().init();

    struct T;

    impl fmt::Display for T {
        fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
            panic!("panic during formatting")
        }
    }

    info!("{}", T);
}
