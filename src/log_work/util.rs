extern crate chrono;
use self::chrono::TimeZone;

use super::*;

pub fn to_date(year: &str, month: &str, day: &str) -> Result<Date>
{
        let year = year.parse::<i32>()?;
        let month = month.parse::<u32>()?;
        let day = day.parse::<u32>()?;
        return Ok(chrono::Local.ymd(year, month, day));
}

pub struct HourMinuteDuration<'a> {
    pub duration: &'a chrono::Duration,
}

impl<'a> std::fmt::Display for HourMinuteDuration<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        return
            write!(f, "{:>2}:{:02}",
                   self.duration.num_hours(),
                   self.duration.num_minutes() % 60);
    }
}

pub struct WorkDuration {
    pub duration: chrono::Duration,
    pub duration_of_day: chrono::Duration,
}

impl std::fmt::Display for WorkDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut remaining_minutes = self.duration.num_minutes();
        let days = remaining_minutes / self.duration_of_day.num_minutes();
        if days > 0 { write!(f, "{:3}d", days)?; }
        else { write!(f, "    ")?; }
        remaining_minutes %= self.duration_of_day.num_minutes();
        let hours = remaining_minutes / 60;
        if hours > 0 { write!(f, " {:2}h", hours)?; }
        else { write!(f, "    ")?; }
        remaining_minutes %= 60;
        let minutes = remaining_minutes;
        if minutes > 0 { write!(f, " {:2}m", minutes)? }
        else { write!(f, "    ")? }
        return write!(f, " ({:>5.2}h)", (self.duration.num_minutes() as f64) / 60.);
    }
}

