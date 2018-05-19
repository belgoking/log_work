#[macro_use] extern crate lazy_static;
extern crate chrono;
extern crate regex;

#[macro_use] extern crate structopt;

use structopt::StructOpt;

/** TODO
 * Collect sub-keys
 * Collect and sum up the entries of one day
 * Create aggregate of several days
 * Collect and sum up the entries of several days
 * Handle flexible required durations per day
 * Handle vacations/half-day-vacations/sickness/holidays/conferences
 */

//use regex::Regex;
//use chrono;
use chrono::TimeZone;

type DateTime = chrono::DateTime<chrono::Local>;
type Date = chrono::Date<chrono::Local>;

#[derive(Eq)]
#[derive(PartialEq)]
#[derive(Debug)]
struct EntryRaw {
    start_ts: DateTime,
    key: String,
    sub_keys: Vec<String>,
    raw_data: String,
}

#[derive(Debug)]
enum EntriesLine<'a> {
    Captures(regex::Captures<'a>),
    Line,
}

#[derive(Debug)]
enum Error {
    IOError(std::io::Error),
    ParseIntError(std::num::ParseIntError),
    InvalidFileNameError{file: std::path::PathBuf},
    ParseDayTypeError{file: String, line_nr: u32},
    TimeNotMonotonicError{file: String, line_nr: u32},
    MissingDateError{file: String},
    UnexpectedDateError{file: String, line_nr: u32,
        expected_date: Date,
        found_date: Date},
}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        use Error;
        match (self, other) {
            (&Error::IOError(_), &Error::IOError(_)) => true,
            (&Error::ParseIntError(_), &Error::ParseIntError(_)) => true,
            (&Error::InvalidFileNameError{file: ref s_file},
             &Error::InvalidFileNameError{file: ref o_file}) =>
                 (s_file==o_file),
            (&Error::ParseDayTypeError{file: ref s_file, line_nr: s_line_nr},
             &Error::ParseDayTypeError{file: ref o_file, line_nr: o_line_nr}) =>
                 (s_file==o_file && s_line_nr==o_line_nr),
            (&Error::TimeNotMonotonicError{file: ref s_file, line_nr: s_line_nr},
             &Error::TimeNotMonotonicError{file: ref o_file, line_nr: o_line_nr}) =>
                 (s_file==o_file && s_line_nr==o_line_nr),
            (&Error::MissingDateError{file: ref s_file},
             &Error::MissingDateError{file: ref o_file}) => (s_file==o_file),
            (&Error::UnexpectedDateError{file: ref s_file, line_nr: s_line_nr,
                                         expected_date: ref s_expected_date,
                                         found_date: ref s_found_date},
             &Error::UnexpectedDateError{file: ref o_file, line_nr: o_line_nr,
                                         expected_date: ref o_expected_date,
                                         found_date: ref o_found_date}) =>
                (s_file==o_file && s_line_nr==o_line_nr &&
                 s_expected_date==o_expected_date && s_found_date==o_found_date),
            _ => return false,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::IOError(ref err) => write!(f, "IOError: {}", err),
            Error::ParseIntError(ref err) => write!(f, "ParseIntError: {}", err),
            Error::InvalidFileNameError{ref file} =>
                write!(f, "InvalidFileNameError: {:?}", file),
            Error::ParseDayTypeError{ref file, ref line_nr} =>
                write!(f, "ParseDayTypeError: {}:{}", file, line_nr),
            Error::TimeNotMonotonicError{ref file, ref line_nr} =>
                write!(f, "TimeNotMonotonicError: {}:{}", file, line_nr),
            Error::MissingDateError{ref file} =>
                write!(f, "MissingDateError: {}", file),
            Error::UnexpectedDateError{
                ref file, ref line_nr,
                ref expected_date, ref found_date} =>
                    write!(f, "UnexpectedDateError: {}:{}: expected={} found={}",
                           file, line_nr, expected_date, found_date),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error { Error::IOError(err) }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Error { Error::ParseIntError(err) }
}


type Result<T> = std::result::Result<T, Error>;

#[derive(Eq)]
#[derive(PartialEq)]
#[derive(Debug)]
struct WorkDay {
    date: Date,
    entries: Vec<EntryRaw>,
    additional_text: String,
}

fn to_date(year: &str, month: &str, day: &str) -> Result<Date>
{
        let year = year.parse::<i32>()?;
        let month = month.parse::<u32>()?;
        let day = day.parse::<u32>()?;
        return Ok(chrono::Local.ymd(year, month, day));
}

impl WorkDay {

    fn read_line(stream: &mut std::io::BufRead) -> Result<(bool, String)>
    {
        let mut line = String::new();
        let num_bytes = stream.read_line(&mut line)?;
        Ok(((num_bytes != 0 && line != "\n"), line))
    }

    fn parse_entries_line<'a>(line: &'a str) -> EntriesLine<'a>
    {
        lazy_static!{
              static ref RE: regex::Regex = regex::Regex::new(r"^-- (\d{4})-(\d{2})-(\d{2}) ([^ ]+ )?(\d{2}):(\d{2}) -- (.*)
?$").expect("Erronuous Regular Expression");
        }
        let cap = RE.captures(line);
        match cap {
            Some(c) => EntriesLine::Captures(c),
            None => EntriesLine::Line,
        }
    }

    fn parse_description(description: &str) -> (String, Vec<String>)
    {
        let description = description.trim_left();
        if description.is_empty() {
            return (String::new(), Vec::new());
        }
        let mut iter = description.split(|c| c == ' ' || c == ':');
        (iter.next().unwrap().to_string(), Vec::new())
    }

    fn parse_entry(year: &str, month: &str, day: &str, hour: &str, minute: &str, desc: &str, raw_data: &str) -> Result<EntryRaw>
    {
        let hour = hour.parse::<u32>()?;
        let minute = minute.parse::<u32>()?;
        let (key, sub_keys) = WorkDay::parse_description(desc);
        let date = to_date(year, month, day)?;
        let start_ts = date.and_hms(hour, minute, 0);

        Ok(EntryRaw{start_ts, key, sub_keys, raw_data: raw_data.to_string()})
    }

    pub fn parse(stream: &mut std::io::BufRead, expected_date: Option<Date>, file: &str) -> Result<WorkDay>
    {
        let mut line_nr = 0u32;
        let date: Option<Date>;
        let (mut non_empty, mut line) = WorkDay::read_line(stream)?;
        while !non_empty {
            line_nr += 1;
            let (tmp_non_empty, tmp_line) = WorkDay::read_line(stream)?;
            non_empty = tmp_non_empty;
            line = tmp_line;
        }
        if line == "" {
            if expected_date.is_none() {
                return Err(Error::MissingDateError{file: file.to_string()});
            }
            return Ok(WorkDay{
                date: expected_date.unwrap(),
                entries: Vec::new(), additional_text: String::new()});
        }
        // handle the entries, if there are some
        let entries = WorkDay::parse_entries(line, stream, &expected_date, file, &mut line_nr)?;
        date = if entries.is_empty() {
            expected_date
        } else {
            Some(entries.get(0).unwrap().start_ts.date())
        };

        // the remaining of the file is the description here we merely check that there is no
        // timestamp
        let mut additional_text = String::new();
        {
            let (_, tmp_line) = WorkDay::read_line(stream)?;
            line = tmp_line;
        }
        while !line.is_empty() {
            line_nr += 1;
            additional_text.push_str(&line[..]);
            let (_, tmp_line) = WorkDay::read_line(stream)?;
            line = tmp_line;
        }
        match date {
            Some(date) => Ok(WorkDay{date, entries, additional_text}),
            None => Err(Error::MissingDateError{file: file.to_string()}),
        }
    }

    fn parse_entries(
        line: String, stream: &mut std::io::BufRead,
        expected_date: &Option<Date>, file: &str, line_nr: &mut u32)
        -> Result<Vec<EntryRaw>>
    {
        let line_match = WorkDay::parse_entries_line(&line);
        let mut entries = Vec::new();
        if let EntriesLine::Captures(c) = line_match {
            let mut entry_raw = WorkDay::parse_entry(&c[1], &c[2], &c[3], &c[5], &c[6], &c[7], &line)?;
            *line_nr += 1;
            let expected_date =
                match *expected_date {
                    None => entry_raw.start_ts.date(),
                    Some(expected_date) => {
                        let found_date = entry_raw.start_ts.date();
                        if expected_date != found_date {
                            return Err(Error::UnexpectedDateError{
                                file: file.to_string(), line_nr: *line_nr,
                                expected_date, found_date});
                        }
                        found_date
                    }
                };
            let mut last_ts = entry_raw.start_ts;
            loop {
                let (non_empty, line) = WorkDay::read_line(stream)?;
                if !non_empty {
                    entries.push(entry_raw);
                    break;
                }
                let line_match = WorkDay::parse_entries_line(&line);
                *line_nr += 1;
                match line_match {
                    EntriesLine::Captures(c) => {
                        entries.push(entry_raw);
                        entry_raw = WorkDay::parse_entry(&c[1], &c[2], &c[3], &c[5], &c[6], &c[7], &line)?;
                        if expected_date != entry_raw.start_ts.date() {
                            return Err(Error::UnexpectedDateError{
                                file: file.to_string(), line_nr: *line_nr,
                                expected_date, found_date: entry_raw.start_ts.date()});
                        }
                        if last_ts > entry_raw.start_ts {
                            return Err(Error::TimeNotMonotonicError{
                                file: file.to_string(), line_nr: *line_nr});
                        }
                        last_ts = entry_raw.start_ts;
                    },
                    EntriesLine::Line => {
                        entry_raw = EntryRaw{
                            raw_data: entry_raw.raw_data + &line,
                            ..entry_raw
                        };
                    },
                }
            }
        };
        return Ok(entries);
    }

    pub fn parse_file(file_name: &std::path::PathBuf) -> Result<WorkDay> {
        lazy_static!{
              static ref RE: regex::Regex = regex::Regex::new(r"^(\d{4})(\d{2})(\d{2})(_.*)\.work$").expect("Erronuous Regular Expression");
        }

        let file_name_str =
            match file_name.to_str() {
                Some(fi) => fi,
                None => return Err(Error::InvalidFileNameError{file: file_name.clone()}),
            };
        let expected_date: Option<Date> = match RE.captures(file_name_str) {
            Some(c) => {
                let y = c[1].parse::<i32>()?;
                let m = c[2].parse::<u32>()?;
                let d = c[3].parse::<u32>()?;
                match chrono::Local.ymd_opt(y, m, d) {
                    chrono::LocalResult::Single(c) => Some(c),
                    _ => None,
                }
            },
            None => None,
        };
        let file = std::fs::File::open(file_name)?;
        let mut fstream = std::io::BufReader::new(file);
        return WorkDay::parse(&mut fstream, expected_date, file_name_str);
    }
}

#[derive(Eq)]
#[derive(PartialEq)]
#[derive(Clone)]
#[derive(Debug)]
pub enum DayType {
    WorkDay,                              // A - Arbeitstag`
    JobTravel{description: String},       // D - Dienstreise
    Sick{description: String},            // K - Krank
    WeekEnd,
    Holiday{name: String},                // F - Feiertag
    Vacation{description: String},        // U - Urlaub
    VacationHalfDay{description: String}, // H - Halber Tag Urlaub
}

fn get_day_type_description(c: &regex::Captures) -> String {
    if let Some(_) = c.get(4) {
        return c[4].to_string();
    }
    // if c[4] is not None c[5] must be there
    return c[5].to_string();
}

//impl std::str::FromStr for DayType {
//    type Err = Error;
//    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
    fn day_type_from_str(s: &str, file_name: &str, line_nr: u32) -> Result<DayType> {
        lazy_static!{
            static ref RE: regex::Regex = regex::Regex::new(r"^([DKFUH]) +((([^:]*):)|([^ ]*)).*$")
                .expect("Erronuous Regular Expression for holiday type parsing");
        }
        match RE.captures(s) {
            Some(c) => {
                match &c[1] {
//                    "A" => return Ok(DayType::WorkDay),
                    "D" => return Ok(DayType::JobTravel{description: get_day_type_description(&c)}),
                    "K" => return Ok(DayType::Sick{description: get_day_type_description(&c)}),
                    "F" => return Ok(DayType::Holiday{name: get_day_type_description(&c)}),
                    "U" => return Ok(DayType::Vacation{description: get_day_type_description(&c)}),
                    "H" => return Ok(DayType::VacationHalfDay{description: get_day_type_description(&c)}),
                    _ => return Err(Error::ParseDayTypeError{file: file_name.to_string(), line_nr}),
                }
            },
            None => {
                return Err(Error::ParseDayTypeError{file: file_name.to_string(), line_nr});
            }
        }
    }
//}

fn parse_required_time(stream: &mut std::io::BufRead, file_name: &str) -> Result<Vec<(Date, DayType)>>
{
    lazy_static!{
          static ref RE: regex::Regex =
              regex::Regex::new(
                  r"^(\d{4})-(\d{2})-(\d{2})(--(\d{4})-(\d{2})-(\d{2}))? -- +(.*?) *
?$"
              ).expect("Erronuous Regular Expression for holiday parsing");
    }
    let mut ret = Vec::<(Date, DayType)>::new();
    let mut line_nr = 0u32;
    loop {
        let mut line = String::new();
        let bytes_read = stream.read_line(&mut line)?;
        if bytes_read == 0 {
            return Ok(ret);
        }
        line_nr += 1;
        if let Some(c) = RE.captures(&line) {
            let start_date = to_date(&c[1], &c[2], &c[3])?;
            let end_date =
                match c.get(4) {
                    Some(_) => to_date(&c[5], &c[6], &c[7])?,
                    None => start_date,
                };
            if start_date > end_date {
                return Err(Error::ParseDayTypeError{file: file_name.to_string(), line_nr});
            }
            let day_type = day_type_from_str(&c[8], file_name, line_nr)?;
            let mut curr_day = start_date;
            while curr_day <= end_date {
                ret.push((curr_day, day_type.clone()));
                curr_day = curr_day.succ();
            }
        }
    }
}

#[derive(Eq)]
#[derive(PartialEq)]
#[derive(Debug)]
struct Day {
    date: Date,
    required_time: std::time::Duration,
    work_day: WorkDay,
}

#[derive(Debug)]
struct Days {
    days: Vec<Day>
}

impl Days {
    pub fn parse_work_files(mut files: Vec<std::path::PathBuf>) -> Vec<Result<WorkDay>>
    {
        files.sort();
        let mut ret: Vec<Result<WorkDay>> = Vec::new();
        ret.reserve_exact(files.len());
        for ref file in files {
            ret.push(WorkDay::parse_file(file));
        }
        return ret;
    }

//    pub fn load(mut files: Vec<String>, _special_dates_file: Option<String>) -> Days
//    {
//        // read work_files
//        // read special_dates
//        // merge both
//        // throw an exception on duplicate days
//    }
}

#[derive(Debug, StructOpt)]
#[structopt(about="Read .work-files and give summaries of worked time.")]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct Opt {
    /// A file containing holidays and vacations
    #[structopt(short="H", long="holidays", parse(from_os_str))]
    holidays: Option<std::path::PathBuf>,

    /// The .work-files
    #[structopt(parse(from_os_str))]
    files: Vec<std::path::PathBuf>,
}

fn main() {

    let opt = Opt::from_args();

    let mut args: Vec<String> = std::env::args().collect();
    args.remove(0);

    for day_raw in Days::parse_work_files(opt.files) {
        println!("Pupupu: {:?}", day_raw);
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use chrono;
    use chrono::TimeZone;

    #[test]
    fn test_parse_error_wrong_day_1()
    {
        let txt: &str = r"-- 2018-05-04 Mo 12:27 -- Foo Bar Baz";
        let txt = txt.as_bytes();
        let mut txt = io::BufReader::new(txt);
        let expected_date = chrono::Local.ymd(2018, 5, 3);
        let entries = super::WorkDay::parse(&mut txt, Some(expected_date), "tst_file");
        let expected_error =
            Err(super::Error::UnexpectedDateError{
                    file: "tst_file".to_string(), line_nr: 1,
                    expected_date,
                    found_date: chrono::Local.ymd(2018, 5, 4)});
        assert_eq!(expected_error, entries);
    }

    #[test]
    fn test_parse_error_wrong_day_2()
    {
        let txt: &str = r"

-- 2018-05-04 Mo 12:27 -- Foo Bar Baz";
        let txt = txt.as_bytes();
        let mut txt = io::BufReader::new(txt);
        let expected_date = chrono::Local.ymd(2018, 5, 3);
        let entries = super::WorkDay::parse(&mut txt, Some(expected_date), "tst_file");
        let expected_error =
            Err(super::Error::UnexpectedDateError{
                    file: "tst_file".to_string(), line_nr: 3,
                    expected_date,
                    found_date: chrono::Local.ymd(2018, 5, 4)});
        assert_eq!(expected_error, entries);
    }

    #[test]
    fn test_parse_error_wrong_day_3()
    {
        let txt: &str = r"
-- 2018-05-03 Mo 12:27 -- Foo Bar Baz
-- 2018-05-04 Mo 12:27 -- Foo Bar Baz";
        let txt = txt.as_bytes();
        let mut txt = io::BufReader::new(txt);
        let expected_date = chrono::Local.ymd(2018, 5, 3);
        let entries = super::WorkDay::parse(&mut txt, Some(expected_date), "tst_file");
        let expected_error =
            Err(super::Error::UnexpectedDateError{
                    file: "tst_file".to_string(), line_nr: 3,
                    expected_date,
                    found_date: chrono::Local.ymd(2018, 5, 4)});
        assert_eq!(expected_error, entries);
    }

    #[test]
    fn test_parse_error_time_non_monotonic()
    {
        let txt: &str = r"-- 2018-05-04 Mo 12:27 -- Foo Bar Baz
-- 2018-05-04 Mo 12:26 -- Foo Bar Baz";
        let mut txt = io::BufReader::new(txt.as_bytes());
        let entries = super::WorkDay::parse(&mut txt, None, "tst_file");
        let expected_error =
            Err(super::Error::TimeNotMonotonicError{
                    file: "tst_file".to_string(), line_nr: 2});
        assert_eq!(expected_error, entries);
    }

    #[test]
    fn test_parse_entries_line_with_empty_lines()
    {
        let txt: &str = r"-- 2018-05-04 Mo 12:27 -- Foo
Bar Baz
-- 2018-05-04 Mo 12:47 -- Bam

Hier kommt jetzt einfach nur noch geblubber
";
        let txt = txt.as_bytes();
        let mut txt = io::BufReader::new(txt);
        let parsed_entries = super::WorkDay::parse(&mut txt, None, "tst_file");
        assert!(parsed_entries.is_ok());

        let expected_entries = vec![
            super::EntryRaw{ start_ts: chrono::Local.ymd(2018, 5, 4).and_hms(12, 27, 0),
                key: "Foo".to_string(), sub_keys: Vec::new(),
                raw_data: "-- 2018-05-04 Mo 12:27 -- Foo\nBar Baz\n".to_string()},
            super::EntryRaw{ start_ts: chrono::Local.ymd(2018, 5, 4).and_hms(12, 47, 0),
                key: "Bam".to_string(), sub_keys: Vec::new(),
                raw_data: "-- 2018-05-04 Mo 12:47 -- Bam\n".to_string()}];
        let expected = super::WorkDay{
            date: chrono::Local.ymd(2018, 5, 4),
            entries: expected_entries,
            additional_text: "Hier kommt jetzt einfach nur noch geblubber\n".to_string()};
        assert_eq!(parsed_entries.unwrap(), expected);
    }

    #[test]
    fn test_parse_required_time_1()
    {
        let txt: &str = r"2018-05-04 -- D Mehrere Worte:";
        let expected = vec![
            (chrono::Local.ymd(2018, 5, 4), super::DayType::JobTravel{description: "Mehrere Worte".to_string()})];
        do_test_parse_required_time(txt, expected);
    }

    #[test]
    fn test_parse_required_time_2()
    {
        let txt: &str = r"2018-05-04--2018-05-05 -- H This is a half day";
        let expected = vec![
            (chrono::Local.ymd(2018, 5, 4), super::DayType::VacationHalfDay{description: "This".to_string()}),
            (chrono::Local.ymd(2018, 5, 5), super::DayType::VacationHalfDay{description: "This".to_string()})];
        do_test_parse_required_time(txt, expected);
    }

    #[test]
    fn test_parse_required_time_3()
    {
        let txt: &str = r"2018-05-04--2018-05-05 -- H This is: a half day";
        let expected = vec![
            (chrono::Local.ymd(2018, 5, 4), super::DayType::VacationHalfDay{description: "This is".to_string()}),
            (chrono::Local.ymd(2018, 5, 5), super::DayType::VacationHalfDay{description: "This is".to_string()})];
        do_test_parse_required_time(txt, expected);
    }

    fn do_test_parse_required_time(txt: &str, expected: Vec<(super::Date, super::DayType)>)
    {
        let txt = txt.as_bytes();
        let mut txt = io::BufReader::new(txt);
        let parsed_entries = super::parse_required_time(&mut txt, "tst_file");
        assert!(parsed_entries.is_ok());

        assert_eq!(parsed_entries.unwrap(), expected);
    }
}
