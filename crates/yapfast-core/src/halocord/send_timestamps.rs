//! Send timestamps: a time written as `<t:13:30>` in a message goes to Discord
//! as a timestamp, which each reader sees in their own time zone. A date
//! (`<t:2026-12-24>`) or both (`<t:2026-12-24 18:00>`) work too, and a time
//! may say am or pm (`<t:6pm>`, `<t:6:30 pm>`).

use std::{
    borrow::Cow,
    sync::atomic::{AtomicBool, Ordering},
};

use chrono::{Local, NaiveDate, NaiveTime, TimeZone};

static ON: AtomicBool = AtomicBool::new(false);

/// Turns the plugin on or off.
pub fn set(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

/// `text` with each written time turned into a Discord timestamp, while the
/// plugin is on.
pub fn wire(text: &str) -> Cow<'_, str> {
    if !ON.load(Ordering::Relaxed) || !text.contains("<t:") {
        return Cow::Borrowed(text);
    }
    Cow::Owned(wire_in(&Local, Local::now().date_naive(), text))
}

fn wire_in<Tz: TimeZone>(zone: &Tz, today: NaiveDate, text: &str) -> String {
    let mut wired = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<t:") {
        wired.push_str(&rest[..start]);
        let after = &rest[start + 3..];
        let found = after
            .find('>')
            .and_then(|end| Some((end, stamp(zone, today, &after[..end])?)));
        match found {
            Some((end, stamp)) => {
                wired.push_str(&stamp);
                rest = &after[end + 1..];
            }
            // Not a time: Discord's own timestamps and anything else stay.
            None => {
                wired.push_str("<t:");
                rest = after;
            }
        }
    }
    wired.push_str(rest);
    wired
}

/// What `written` becomes: a short time for a time on its own, a long date
/// for a date, and both for both. None when it isn't one.
fn stamp<Tz: TimeZone>(zone: &Tz, today: NaiveDate, written: &str) -> Option<String> {
    let written = written.trim();
    let (date, time, style) = match parse_date(written) {
        Some(date) => (date, NaiveTime::MIN, 'D'),
        None => match written
            .split_once(' ')
            .map(|(date, time)| (parse_date(date), time))
        {
            Some((Some(date), time)) => (date, parse_time(time)?, 'f'),
            _ => (today, parse_time(written)?, 't'),
        },
    };
    let seconds = zone
        .from_local_datetime(&date.and_time(time))
        .earliest()?
        .timestamp();
    Some(format!("<t:{seconds}:{style}>"))
}

fn parse_date(written: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(written.trim(), "%Y-%m-%d").ok()
}

fn parse_time(written: &str) -> Option<NaiveTime> {
    let lower = written.trim().to_ascii_lowercase();
    let (clock, pm) = match (lower.strip_suffix("am"), lower.strip_suffix("pm")) {
        (Some(clock), _) => (clock.trim(), Some(false)),
        (_, Some(clock)) => (clock.trim(), Some(true)),
        _ => (lower.as_str(), None),
    };
    let (hours, minutes) = match clock.split_once(':') {
        Some((hours, minutes)) if minutes.len() == 2 => {
            (hours.parse::<u32>().ok()?, minutes.parse::<u32>().ok()?)
        }
        None if pm.is_some() => (clock.parse::<u32>().ok()?, 0),
        _ => return None,
    };
    let hours = match pm {
        Some(_) if !(1..=12).contains(&hours) => return None,
        Some(pm) => hours % 12 + if pm { 12 } else { 0 },
        None => hours,
    };
    NaiveTime::from_hms_opt(hours, minutes, 0)
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, Utc};

    use super::*;

    /// Seconds at a UTC time on 23 November 2026.
    fn at(hours: u32, minutes: u32) -> i64 {
        Utc.with_ymd_and_hms(2026, 11, 23, hours, minutes, 0)
            .unwrap()
            .timestamp()
    }

    #[test]
    fn written_times_become_discord_timestamps() {
        // An hour ahead of UTC, so 13:30 here is 12:30 UTC.
        let zone = FixedOffset::east_opt(3600).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 11, 23).unwrap();
        let wire = |text| wire_in(&zone, today, text);
        assert_eq!(
            wire("meet at <t:13:30>?"),
            format!("meet at <t:{}:t>?", at(12, 30))
        );
        assert_eq!(
            wire("<t:6pm> or <t:6:30 am>"),
            format!("<t:{}:t> or <t:{}:t>", at(17, 0), at(5, 30))
        );
        assert_eq!(wire("<t:12am>"), format!("<t:{}:t>", at(0, 0) - 3600));
        assert_eq!(wire("<t:2026-11-23 18:00>"), format!("<t:{}:f>", at(17, 0)));
        assert_eq!(wire("<t:2026-11-23>"), format!("<t:{}:D>", at(0, 0) - 3600));
        for kept in [
            "<t:1700000000:R>",
            "<t:1700000000>",
            "<t:25:00>",
            "<t:13pm>",
            "<t:1:5>",
            "<t:13:30",
            "no times here",
        ] {
            assert_eq!(wire(kept), kept);
        }
    }

    #[test]
    fn nothing_changes_while_off() {
        assert_eq!(wire("<t:13:30>"), "<t:13:30>");
    }
}
