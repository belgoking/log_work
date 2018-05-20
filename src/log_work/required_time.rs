extern crate chrono;
extern crate regex;

use super::*;
use self::util;
use std;


#[derive(Eq)]
#[derive(PartialEq)]
#[derive(Clone)]
#[derive(Debug)]
pub enum DayType {
    WorkDay,                                // A - Arbeitstag
    JobTravel{description: String},         // D - Dienstreise
    Sick{description: String},              // K - Krank
    WeekEnd,
    Holiday{name: String},                  // F - Feiertag
    Vacation{description: String},          // U - Urlaub
    VacationHalfDay{description: String},   // H - Halber Tag Urlaub
    OvertimeReduction{description: String}, // Ü - Überstundenabbau
}

#[derive(Eq, PartialEq, PartialOrd, Clone, Debug)]
enum DayTypeClass {
    Work,
    Vacation,
    WeekendAndHolidays,
}

impl DayType {
    fn to_day_type_class(&self) -> DayTypeClass {
        match *self {
            DayType::WorkDay => return DayTypeClass::Work,
            DayType::OvertimeReduction{description: _} => return DayTypeClass::Work,
            DayType::WeekEnd => return DayTypeClass::WeekendAndHolidays,
            DayType::JobTravel{description: _} => return DayTypeClass::Work,
            DayType::Sick{description: _} => return DayTypeClass::Work,
            DayType::Holiday{name: _} => return DayTypeClass::WeekendAndHolidays,
            DayType::Vacation{description: _} => return DayTypeClass::Vacation,
            DayType::VacationHalfDay{description: _} => return DayTypeClass::Vacation,
        };
    }
}

#[derive(Eq,PartialEq,Clone,Debug)]
pub struct DayTypeEntry {
    date: Date,
    day_type: DayType,
    given_as_range: bool,
    line_nr: u32,
}


fn get_day_type_description(c: &regex::Captures) -> String {
    if let Some(_) = c.get(4) {
        return c[4].to_string();
    }
    // if c[4] is not None c[5] must be there
    return c[5].to_string();
}

fn check_day_types(orig: &DayTypeEntry, new: &DayTypeEntry) -> Result<()>
{
    if orig.date != new.date {
        return Ok(());
    }
    if !new.given_as_range && new.day_type.to_day_type_class() != DayTypeClass::WeekendAndHolidays {
        return Err(Error::DuplicateDateError{file: "".to_string() /*new.file.clone()*/, line_nr: new.line_nr});
    }
    if !orig.given_as_range && orig.day_type.to_day_type_class() != DayTypeClass::WeekendAndHolidays {
        return Err(Error::DuplicateDateError{file: "".to_string() /*orig.file.clone()*/, line_nr: orig.line_nr});
    }
    return Ok(());
}

fn consolidate_required_time(raw_entries: &Vec<DayTypeEntry>, start_date: &Date, end_date: &Date) -> Result<()>
{
    let mut map: std::collections::BTreeMap<Date, DayTypeEntry> = std::collections::BTreeMap::new();
    for ref raw_entry in raw_entries {
        let mut found = false;
        if let Some(ref mut old_entry) = map.get(&raw_entry.date) {
            found = true;
            check_day_types(old_entry, raw_entry)?;
            if raw_entry.day_type.to_day_type_class() > old_entry.day_type.to_day_type_class() {
                let mut cloned = raw_entry.clone();
                *old_entry = cloned;
            }
//        } else {
//            let cloned = (*raw_entry).clone();
//            map.insert(raw_entry.date.clone(), cloned);
        };
        if !found {
            let cloned = (*raw_entry).clone();
            map.insert(raw_entry.date.clone(), cloned);
        }

    }
    return Ok(());
}

pub fn parse_required_time_file(file_name: &std::path::PathBuf) -> Result<Vec<DayTypeEntry>>
{
    let file = std::fs::File::open(&file_name)?;
    let mut fstream = std::io::BufReader::new(file);
    let file_name_str =
        match file_name.to_str() {
            Some(fi) => fi,
            None     => return Err(Error::InvalidFileNameError{file: file_name.clone()}),
        };
    let ret = parse_required_time(&mut fstream, file_name_str)?;
    return Ok(ret);
}


fn day_type_from_str(s: &str, file_name: &str, line_nr: u32) -> Result<DayType> {
    lazy_static!{
        static ref RE: regex::Regex = regex::Regex::new(r"^([DKFUHÜ]) +((([^:]*):)|([^ ]*)).*$")
            .expect("Erronuous Regular Expression for holiday type parsing");
    }
    match RE.captures(s) {
        Some(c) => {
            match &c[1] {
                "D" => return Ok(DayType::JobTravel{description: get_day_type_description(&c)}),
                "K" => return Ok(DayType::Sick{description: get_day_type_description(&c)}),
                "F" => return Ok(DayType::Holiday{name: get_day_type_description(&c)}),
                "U" => return Ok(DayType::Vacation{description: get_day_type_description(&c)}),
                "H" => return Ok(DayType::VacationHalfDay{description: get_day_type_description(&c)}),
                "Ü" => return Ok(DayType::OvertimeReduction{description: get_day_type_description(&c)}),
                _ => return Err(Error::ParseDayTypeError{file: file_name.to_string(), line_nr}),
            }
        },
        None => {
            return Err(Error::ParseDayTypeError{file: file_name.to_string(), line_nr});
        }
    }
}

pub fn parse_required_time(stream: &mut std::io::BufRead, file_name: &str) -> Result<Vec<DayTypeEntry>>
{
    lazy_static!{
          static ref RE: regex::Regex =
              regex::Regex::new(
                  r"^(\d{4})-(\d{2})-(\d{2})(--(\d{4})-(\d{2})-(\d{2}))? -- +(.*?) *
?$"
              ).expect("Erronuous Regular Expression for holiday parsing");
    }
    let mut ret = Vec::<DayTypeEntry>::new();
    let mut line_nr = 0u32;
    loop {
        let mut line = String::new();
        let bytes_read = stream.read_line(&mut line)?;
        if bytes_read == 0 {
            return Ok(ret);
        }
        line_nr += 1;
        if let Some(c) = RE.captures(&line) {
            let start_date = util::to_date(&c[1], &c[2], &c[3])?;
            let (end_date, given_as_range) =
                match c.get(4) {
                    Some(_) => (util::to_date(&c[5], &c[6], &c[7])?, true),
                    None => (start_date, false),
                };
            if start_date > end_date {
                return Err(Error::ParseDayTypeError{file: file_name.to_string(), line_nr});
            }
            let day_type = day_type_from_str(&c[8], file_name, line_nr)?;
            let mut curr_day = start_date;
            while curr_day <= end_date {
                ret.push(DayTypeEntry{date: curr_day, day_type: day_type.clone(), given_as_range, line_nr});
                curr_day = curr_day.succ();
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use std::io;
    use self::chrono;
    use self::chrono::TimeZone;
    use super::*;

    #[test]
    fn test_parse_required_time_1()
    {
        let txt: &str = r"2018-05-04 -- D Mehrere Worte:";
        let expected = Ok(vec![
            DayTypeEntry{date: chrono::Local.ymd(2018, 5, 4), day_type: DayType::JobTravel{description: "Mehrere Worte".to_string()}, given_as_range: false, line_nr: 1}]);
        do_test_parse_required_time(txt, expected);
    }

    #[test]
    fn test_parse_required_time_2()
    {
        let txt: &str = r"2018-05-04--2018-05-05 -- H This is a half day";
        let expected = Ok(vec![
            DayTypeEntry{date: chrono::Local.ymd(2018, 5, 4), day_type: DayType::VacationHalfDay{description: "This".to_string()}, given_as_range: true, line_nr: 1},
            DayTypeEntry{date: chrono::Local.ymd(2018, 5, 5), day_type: DayType::VacationHalfDay{description: "This".to_string()}, given_as_range: true, line_nr: 1}]);
        do_test_parse_required_time(txt, expected);
    }

    #[test]
    fn test_parse_required_time_3()
    {
        let txt: &str = r"2018-05-04--2018-05-05 -- K This is: a sickness day";
        let expected = Ok(vec![
            DayTypeEntry{date: chrono::Local.ymd(2018, 5, 4), day_type: DayType::Sick{description: "This is".to_string()}, given_as_range: true, line_nr: 1},
            DayTypeEntry{date: chrono::Local.ymd(2018, 5, 5), day_type: DayType::Sick{description: "This is".to_string()}, given_as_range: true, line_nr: 1}]);
        do_test_parse_required_time(txt, expected);
    }

    #[test]
    fn test_parse_required_time_4()
    {
        let txt: &str = r"2018-05-04 -- F This is: a holiday
2018-05-07 -- U A vacation day
2018-05-06 -- Ü Brückentag";
        let expected = Ok(vec![
            DayTypeEntry{date: chrono::Local.ymd(2018, 5, 4), day_type: DayType::Holiday{name: "This is".to_string()}, given_as_range: false, line_nr: 1},
            DayTypeEntry{date: chrono::Local.ymd(2018, 5, 7), day_type: DayType::Vacation{description: "A".to_string()}, given_as_range: false, line_nr: 2},
            DayTypeEntry{date: chrono::Local.ymd(2018, 5, 6), day_type: DayType::OvertimeReduction{description: "Brückentag".to_string()}, given_as_range: false, line_nr: 3}]);
        do_test_parse_required_time(txt, expected);
    }

    #[test]
    fn test_parse_required_time_error_1()
    {
        let txt: &str = r"2018-05-04 -- F This is: a holiday
2018-05-07 -- u A half day";
        let expected = Err(Error::ParseDayTypeError{file: "tst_file".to_string(), line_nr: 2});
        do_test_parse_required_time(txt, expected);
    }

    #[test]
    fn test_parse_required_time_error_2()
    {
        let txt: &str = r"2018-05-04--2018-04-05 -- F This is: a holiday";
        let expected = Err(Error::ParseDayTypeError{file: "tst_file".to_string(), line_nr: 1});
        do_test_parse_required_time(txt, expected);
    }

    fn do_test_parse_required_time(txt: &str, expected: Result<Vec<DayTypeEntry>>)
    {
        let txt = txt.as_bytes();
        let mut txt = io::BufReader::new(txt);
        let parsed_entries = parse_required_time(&mut txt, "tst_file");

        assert_eq!(parsed_entries, expected);
    }
}

