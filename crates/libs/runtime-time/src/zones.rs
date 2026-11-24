//! Bundled IANA zones for callers that still exchange Chrono date types.

use chrono::{
    Datelike, FixedOffset, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeZone,
    Timelike,
};
use std::{fmt, str::FromStr};
use tz::datetime::FoundDateTimeKind;

pub use tzdb::TZ_NAMES;

/// A named IANA time zone from the bundled time zone database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedTimeZone(&'static str);

pub const UTC: NamedTimeZone = NamedTimeZone("UTC");
pub const SHANGHAI: NamedTimeZone = NamedTimeZone("Asia/Shanghai");
pub const NEW_YORK: NamedTimeZone = NamedTimeZone("America/New_York");
pub const BERLIN: NamedTimeZone = NamedTimeZone("Europe/Berlin");

impl NamedTimeZone {
    pub fn name(self) -> &'static str {
        self.0
    }

    fn zone(self) -> tz::TimeZoneRef<'static> {
        tzdb::tz_by_name(self.0).expect("validated IANA time zone")
    }

    fn offset_at(self, timestamp: i64) -> NamedOffset {
        let local_type = self
            .zone()
            .find_local_time_type(timestamp)
            .expect("bundled IANA time zone covers Chrono date range");
        let offset = FixedOffset::east_opt(local_type.ut_offset())
            .expect("bundled IANA offset fits in a Chrono fixed offset");
        NamedOffset { zone: self, offset, designation: local_type.time_zone_designation() }
    }
}

impl fmt::Display for NamedTimeZone {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownTimeZone;

impl fmt::Display for UnknownTimeZone {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("unknown IANA time zone")
    }
}

impl std::error::Error for UnknownTimeZone {}

impl FromStr for NamedTimeZone {
    type Err = UnknownTimeZone;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        TZ_NAMES
            .iter()
            .copied()
            .find(|candidate| *candidate == name)
            .map(Self)
            .ok_or(UnknownTimeZone)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedOffset {
    zone: NamedTimeZone,
    offset: FixedOffset,
    designation: &'static str,
}

impl Offset for NamedOffset {
    fn fix(&self) -> FixedOffset {
        self.offset
    }
}

impl fmt::Display for NamedOffset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.designation.is_empty() {
            self.offset.fmt(formatter)
        } else {
            formatter.write_str(self.designation)
        }
    }
}

impl TimeZone for NamedTimeZone {
    type Offset = NamedOffset;

    fn from_offset(offset: &Self::Offset) -> Self {
        offset.zone
    }

    fn offset_from_local_date(&self, date: &NaiveDate) -> LocalResult<Self::Offset> {
        let first = self.offset_from_local_datetime(&date.and_time(NaiveTime::MIN));
        let end = date.and_hms_opt(23, 59, 59).expect("valid end of day");
        let last = self.offset_from_local_datetime(&end);
        match (first, last) {
            (single @ LocalResult::Single(_), _) | (_, single @ LocalResult::Single(_)) => single,
            (LocalResult::Ambiguous(offset, _), _) | (_, LocalResult::Ambiguous(offset, _)) => {
                LocalResult::Single(offset)
            }
            (LocalResult::None, LocalResult::None) => LocalResult::None,
        }
    }

    fn offset_from_local_datetime(&self, date: &NaiveDateTime) -> LocalResult<Self::Offset> {
        let Ok(matches) = tz::DateTime::find(
            date.year(),
            date.month() as u8,
            date.day() as u8,
            date.hour() as u8,
            date.minute() as u8,
            date.second() as u8,
            0,
            self.zone(),
        ) else {
            return LocalResult::None;
        };
        let mut offsets =
            matches.into_inner().into_iter().filter_map(|candidate| match candidate {
                FoundDateTimeKind::Normal(date) => Some(self.offset_at(date.unix_time())),
                FoundDateTimeKind::Skipped { .. } => None,
            });
        let Some(first) = offsets.next() else {
            return LocalResult::None;
        };
        match offsets.next_back() {
            Some(last) => LocalResult::Ambiguous(first, last),
            None => LocalResult::Single(first),
        }
    }

    fn offset_from_utc_date(&self, date: &NaiveDate) -> Self::Offset {
        self.offset_from_utc_datetime(&date.and_time(NaiveTime::MIN))
    }

    fn offset_from_utc_datetime(&self, date: &NaiveDateTime) -> Self::Offset {
        self.offset_at(date.and_utc().timestamp())
    }
}

#[cfg(test)]
#[path = "zones_tests.rs"]
mod tests;
