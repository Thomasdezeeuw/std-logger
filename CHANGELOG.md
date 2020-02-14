# Changelog

## v0.3.5

* **BREAKING** updated minimum supported Rust version to 1.33.
* Added `request!` macro, a convenient way to log requests.
* Added unstable support to print key-value pairs, using `log`'s "kv_unstable"
  feature.
* Dropped `libc` as dependency.

## v0.3.4

* Changed examples to use Rust 2018.
* Add log filtering by using `LOG_TARGET`.
* Write directly to stdout/stderr, not using the standard library's locks.

## v0.3.3

* Only call write once, before it would use `write_all` which calls `write` in
  a loop. Now it errors (panics) if the entire message can't be written in one
  write call.
* Updates to Rust 2018.
* **BREAKING** Requires Rust 1.31 or higher to compile.

## v0.3.2

* Added `try_init`, a version of `init` that doesn't panic.
* Document that `init` panics if it fails to initialise the logger, no
  functional change but the panic was never documented.

## v0.3.1

* Improve documentation.
* Improve performance by using a thread-local buffer for printing messages.

## v0.3.0

* Update to log 0.4.1.
* Update to log-panics 2.0.
* Update to lazy_static 1.0 (test only).
* Expanded documentation.
* Don't log anything in `init`.

## v0.2.0

* An intial debug message is now printed to test the inital setup of the logger.
* Docs where added for changing the severity level (no code change).
* Changed the timestamp format to RFC 3339.
* Improved logging with timestamp performance.
* The `catch-panic` feature was renamed to `log-panic`.
* Backtraces are now logged (using the `log-panic` feature).
* Panics are now logged with a `panic` target.

## v0.1.0

Initial release.
