#[cfg(feature = "log-panic")]
fn main() {
    use log::info;

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
