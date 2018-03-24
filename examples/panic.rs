// Copyright 2017-2018 Thomas de Zeeuw
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option. This file may not be
// used, copied, modified, or distributed except according to those terms.

#[macro_use]
extern crate log;
extern crate std_logger;

#[cfg(feature = "log-panic")]
fn main() {
    // Initialize the logger.
    std_logger::init();

    // This will only be logged when using a environment variable to set the log
    // level to info or lower, e.g. using `LOG_LEVEL=info`.
    info!("going to panic in a moment");

    // This panic will be logging properly to standard error.
    // Something along these lines:
    // 2017-08-04T13:52:22.336819Z [ERROR] panic: thread 'main' panicked at 'oops': panic.rs:24
    panic!("oops");
}

#[cfg(not(feature = "log-panic"))]
fn main() {
    panic!("enable the `log-panic` feature to run this example");
}
