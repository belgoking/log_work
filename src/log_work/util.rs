extern crate chrono;

use super::*;

pub fn to_date(year: &str, month: &str, day: &str) -> Result<Date> {
    let year = year.parse::<i32>()?;
    let month = month.parse::<u32>()?;
    let day = day.parse::<u32>()?;
    Date::from_ymd_opt(year, month, day).ok_or_else(|| Error::ParseDay)
}

pub struct HourMinuteDuration<'a> {
    pub duration: &'a chrono::Duration,
}

impl<'a> std::fmt::Display for HourMinuteDuration<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:>2}:{:02}",
            self.duration.num_hours(),
            self.duration.num_minutes() % 60
        )
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
        remaining_minutes %= self.duration_of_day.num_minutes();
        let hours = remaining_minutes / 60;
        remaining_minutes %= 60;
        let minutes = remaining_minutes;

        let total_minutes = (self.duration.num_minutes() as f64) / 60.;

        let txt = if days > 0 {
            format!(
                "{}d {}h {:2}m ({:>5.2}h)",
                days, hours, minutes, total_minutes
            )
        } else if hours > 0 {
            format!("{}h {:2}m ({:>5.2}h)", hours, minutes, total_minutes)
        } else {
            format!("{:2}m ({:>5.2}h)", minutes, total_minutes)
        };

        f.pad(txt.as_str())
    }
}
