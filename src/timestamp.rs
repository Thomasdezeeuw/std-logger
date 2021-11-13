use std::time::{Duration, SystemTime};

/// Timstamp for humans.
pub(crate) struct Timestamp {
    pub(crate) year: u16,
    pub(crate) month: u8,
    pub(crate) day: u8,
    pub(crate) hour: u8,
    pub(crate) min: u8,
    pub(crate) sec: u8,
    pub(crate) micro: u32,
}

#[cfg(feature = "timestamp")]
impl Timestamp {
    pub(crate) fn now() -> Timestamp {
        Timestamp::from(SystemTime::now())
    }

    /// # Notes
    ///
    /// This only works for days later then 2001.
    // NOTE: pub for testing.
    pub(crate) fn from(time: SystemTime) -> Timestamp {
        // Ported from musl, original source:
        // <https://git.musl-libc.org/cgit/musl/tree/src/time/__secs_to_tm.c>.

        /// 2000-03-01 (mod 400 year, immediately after feb29).
        const LEAPOCH: u64 = 946684800 + 86400 * (31 + 29);
        const DAYS_PER_400Y: u64 = 365 * 400 + 97;
        const DAYS_PER_100Y: u64 = 365 * 100 + 24;
        const DAYS_PER_4Y: u64 = 365 * 4 + 1;
        const DAYS_IN_MONTH: [u64; 12] = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];

        let diff = time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::new(0, 0));
        let secs_since_epoch = diff.as_secs();

        let secs = secs_since_epoch - LEAPOCH;
        let days = secs / 86400;
        let remsecs = secs % 86400;

        let qc_cycles = days / DAYS_PER_400Y;
        let mut remdays = days % DAYS_PER_400Y;

        let mut c_cycles = remdays / DAYS_PER_100Y;
        if c_cycles == 4 {
            c_cycles -= 1;
        }
        remdays -= c_cycles * DAYS_PER_100Y;

        let mut q_cycles = remdays / DAYS_PER_4Y;
        if q_cycles == 25 {
            q_cycles -= 1;
        }
        remdays -= q_cycles * DAYS_PER_4Y;

        let mut remyears = remdays / 365;
        if remyears == 4 {
            remyears -= 1
        }
        remdays -= remyears * 365;

        let mut year = remyears + (4 * q_cycles) + (100 * c_cycles) + (400 * qc_cycles);

        // Determine the month of the year based on the remaining days
        // (`remdays`).
        let mut month = 0;
        for days_in_month in DAYS_IN_MONTH {
            if days_in_month > remdays {
                break;
            }
            remdays -= days_in_month;
            month += 1;
        }
        if month >= 10 {
            month -= 12;
            year += 1;
        }

        Timestamp {
            year: (year + 100 + 1900) as u16,
            month: (month + 2 + 1) as u8,
            day: (remdays + 1) as u8,
            hour: (remsecs / 3600) as u8,
            min: (remsecs / 60 % 60) as u8,
            sec: (remsecs % 60) as u8,
            micro: diff.subsec_micros(),
        }
    }
}
