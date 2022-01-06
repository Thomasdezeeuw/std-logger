# Changelog

## v0.4.4

* Fixes logging of messages using quotes using GCP (gcloud) formatting
  (https://github.com/Thomasdezeeuw/std-logger/commit/0c39b7a3c40d07367b66cc0a937793f75a93a35a).

## v0.4.3

* Fixes severity names using GCP (gcloud) formatting
  (https://github.com/Thomasdezeeuw/std-logger/commit/ae98def82d7f9db3a0cce51da16d7614eeec18de).

## v0.4.2

* `Config` type support multiple logging formats
  (https://github.com/Thomasdezeeuw/std-logger/commit/2abba6460179296791fe2619fe1f3b2988452eaf).
* Added JSON formatting for GCP (gcloud)
  (https://github.com/Thomasdezeeuw/std-logger/commit/b3059695ff22851c7d47986e58c58599b45d4e0d).
* Added `Config::with_kvs`
  (https://github.com/Thomasdezeeuw/std-logger/commit/492646c196993877741a7260578d29c89c8b969a).
* Removed libc dependency
  (https://github.com/Thomasdezeeuw/std-logger/commit/fc74144f7a1a1b43feaa582d5f3aad2b1bc39f41).

## v0.4.1

* Fixes handling panics during logging
  (https://github.com/Thomasdezeeuw/std-logger/commit/770219029a9a96b08a1d7c5b1aca8ad948216784).
* Always log panics, using the "panic" target regardless of set `LOG_TARGET`
  (https://github.com/Thomasdezeeuw/std-logger/commit/be2c2e3ce6bb4a6bfda7cd7a2e9d30ce41b16563).

## v0.4.0

* **BREAKING** changes the log output to be based on [logfmt]
  (https://github.com/Thomasdezeeuw/std-logger/commit/ba1b53c1b940aebe8dc31acac12b6a2e6f412738).
* Adds logging of file name and line when debug logging is enabled
  (https://github.com/Thomasdezeeuw/std-logger/commit/f96425adbac0e183eadf05fabd52eb74c1b15ad0).
* Adds a `nightly` feature that, if enabled, uses nightly-only features from the
  standard library
  (https://github.com/Thomasdezeeuw/std-logger/commit/8e4f1512ed785f2c84caaa54de5c8aaf559a09b0).
* Uses `std::backtrace` if `log-panic` and `nightly` is enabled instead of the
  log-panics crate
  (https://github.com/Thomasdezeeuw/std-logger/commit/61b46506d769c57002da95adfadcda3aaec1bf1d).
  This changes the logged file line and number to be that of the panic
  (https://github.com/Thomasdezeeuw/std-logger/commit/5b27bea25f0bb654f85092eefad658652d41d90c).
* Removed chrono depdency
  (https://github.com/Thomasdezeeuw/std-logger/commit/81a6e325c8d6a0fe10738d032da2899c4fc4db03).

[logfmt]: https://www.brandur.org/logfmt

## v0.3.6

* Fix `request!` macro.

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
