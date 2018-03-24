// Copyright 2017-2018 Thomas de Zeeuw
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option. This file may not be
// used, copied, modified, or distributed except according to those terms.

#![feature(test)]

extern crate chrono;
extern crate test;

use test::Bencher;

#[bench]
fn time_formatting_with_chrono_format(b: &mut Bencher) {
    use chrono::format::Pad::Zero;
    use chrono::format::Item::{self, Fixed, Literal, Numeric};
    use chrono::format::Numeric::{Day, Hour, Minute, Month, Second, Year};
    use chrono::format::Fixed::Nanosecond6;
    const FORMAT_ITEMS: [Item<'static>; 13] = [
        Numeric(Year, Zero),
        Literal("-"),
        Numeric(Month, Zero),
        Literal("-"),
        Numeric(Day, Zero),
        Literal("T"),
        Numeric(Hour, Zero),
        Literal(":"),
        Numeric(Minute, Zero),
        Literal(":"),
        Numeric(Second, Zero),
        Fixed(Nanosecond6),
        Literal("Z"),
        // We're always printing a UTC timezone, no need to print the offset.
    ];

    let timestamp = chrono::Utc::now();
    b.iter(|| {
        format!("{} [{}] {}: {}\n",
            timestamp.format_with_items(FORMAT_ITEMS.iter().cloned()),
            "INFO", "target", "Some message");
    })
}

#[bench]
fn time_formatting_with_manual_format(b: &mut Bencher) {
    use chrono::{Datelike, Timelike};
    let timestamp = chrono::Utc::now();
    b.iter(|| {
        format!("{:004}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z [{}] {}: {}\n",
            timestamp.year(), timestamp.month(), timestamp.day(),
            timestamp.hour(), timestamp.minute(), timestamp.second(),
            timestamp.nanosecond(), "INFO", "target", "Some message");
    })
}
