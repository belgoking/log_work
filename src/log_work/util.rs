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


