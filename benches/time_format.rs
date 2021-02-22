use criterion::{criterion_group, criterion_main, Criterion};

fn chrono_format(c: &mut Criterion) {
    use chrono::format::Fixed::Nanosecond6;
    use chrono::format::Item::{self, Fixed, Literal, Numeric};
    use chrono::format::Numeric::{Day, Hour, Minute, Month, Second, Year};
    use chrono::format::Pad::Zero;
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

    c.bench_function("chrono format", |b| {
        let timestamp = chrono::Utc::now();
        let mut out = String::new();
        b.iter(|| {
            out = format!(
                "{} [{}] {}: {}\n",
                timestamp.format_with_items(FORMAT_ITEMS.iter().cloned()),
                "INFO",
                "target",
                "Some message"
            );
        })
    });
}

fn manual_format(c: &mut Criterion) {
    use chrono::{Datelike, Timelike};

    c.bench_function("manual format", |b| {
        let timestamp = chrono::Utc::now();
        let mut out = String::new();
        b.iter(|| {
            out = format!(
                "{:004}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z [{}] {}: {}\n",
                timestamp.year(),
                timestamp.month(),
                timestamp.day(),
                timestamp.hour(),
                timestamp.minute(),
                timestamp.second(),
                timestamp.nanosecond(),
                "INFO",
                "target",
                "Some message"
            );
        })
    });
}

fn libc_format(c: &mut Criterion) {
    use std::mem::MaybeUninit;
    use std::time::{Duration, SystemTime};

    c.bench_function("libc format", |b| {
        let now = SystemTime::now();
        let diff = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::new(0, 0));

        let mut out = String::new();
        b.iter(|| {
            let mut tm = MaybeUninit::uninit();
            let secs_since_epoch = diff.as_secs() as i64;
            let tm = unsafe { libc::gmtime_r(&secs_since_epoch, tm.as_mut_ptr()) };
            let (year, month, day, hour, min, sec) = match unsafe { tm.as_ref() } {
                Some(tm) => (
                    tm.tm_year + 1900,
                    tm.tm_mon + 1,
                    tm.tm_mday,
                    tm.tm_hour,
                    tm.tm_min,
                    tm.tm_sec,
                ),
                None => (0, 0, 0, 0, 0, 0),
            };
            let nanos = diff.subsec_nanos();

            out = format!(
                "{:004}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z [{}] {}: {}\n",
                year, month, day, hour, min, sec, nanos, "INFO", "target", "Some message"
            );
        })
    });
}

criterion_group!(time_format, chrono_format, manual_format, libc_format);
criterion_main!(time_format);
