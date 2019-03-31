use criterion::{Criterion, criterion_group, criterion_main};

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
        b.iter(|| {
            format!("{} [{}] {}: {}\n",
                timestamp.format_with_items(FORMAT_ITEMS.iter().cloned()),
                "INFO", "target", "Some message");
        })
    });
}

fn manual_format(c: &mut Criterion) {
    use chrono::{Datelike, Timelike};

    c.bench_function("manual format", |b| {
        let timestamp = chrono::Utc::now();
        b.iter(|| {
            format!("{:004}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z [{}] {}: {}\n",
            timestamp.year(), timestamp.month(), timestamp.day(),
            timestamp.hour(), timestamp.minute(), timestamp.second(),
            timestamp.nanosecond(), "INFO", "target", "Some message");
        })
    });
}

criterion_group!(time_format, chrono_format, manual_format);
criterion_main!(time_format);
