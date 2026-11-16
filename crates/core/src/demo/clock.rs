use std::time::{SystemTime, UNIX_EPOCH};

const DAY: u64 = 86_400;

pub(super) fn iso(time: SystemTime) -> String {
    let since = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let (days, seconds) = (since.as_secs() / DAY, since.as_secs() % DAY);
    let (year, month, day) = civil(days);
    let (hour, minute, second) = (seconds / 3_600, seconds / 60 % 60, seconds % 60);
    let millis = since.subsec_millis();
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

fn civil(days: u64) -> (u64, u64, u64) {
    let shifted = days + 719_468;
    let (era, of_era) = (shifted / 146_097, shifted % 146_097);
    let year_of_era = (of_era - of_era / 1_460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let from_march = (5 * of_year + 2) / 153;
    let day = of_year - (153 * from_march + 2) / 5 + 1;
    let month = if from_march < 10 { from_march + 3 } else { from_march - 9 };
    (era * 400 + year_of_era + u64::from(month <= 2), month, day)
}

#[cfg(test)]
mod tests;
