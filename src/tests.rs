// Copyright 2017 Thomas de Zeeuw
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option. This file may not be
// used, copied, modified, or distributed except according to those terms.

use super::*;

#[test]
fn should_get_the_correct_log_level_from_env() {
    let tests = vec![
        ("LOG", "TRACE", LogLevelFilter::Trace),
        ("LOG", "ERROR", LogLevelFilter::Error),
        ("LOG_LEVEL", "ERROR", LogLevelFilter::Error),
        ("LOG_LEVEL", "DEBUG", LogLevelFilter::Debug),
        ("TRACE", "1", LogLevelFilter::Trace),
        ("DEBUG", "1", LogLevelFilter::Debug),
    ];

    for test in tests {
        env::set_var(test.0, test.1);

        let want = test.2;
        let got = get_max_level();
        assert_eq!(want, got);

        env::remove_var(test.0);
    }
}
