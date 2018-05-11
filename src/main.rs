#[macro_use] extern crate lazy_static;
extern crate chrono;
extern crate regex;

//use regex::Regex;
//use chrono;
use chrono::TimeZone;

type DateTime = chrono::DateTime<chrono::Local>;
type Date = chrono::Date<chrono::Local>;

#[derive(PartialEq)]
#[derive(Debug)]
struct EntryRaw {
    start_ts: DateTime,
    key: String,
    sub_keys: Vec<String>,
    raw_data: String,
}

#[derive(PartialEq)]
#[derive(Debug)]
struct DayRaw {
    date: Date,
    entries: Vec<EntryRaw>,
    additional_text: String,
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
    TimeNotMonotonicError{ file: String, line_nr: u32 },
    MissingDateError{ file: String },
    UnexpectedDateError{ file: String, line_nr: u32,
        expected_date: Date,
        found_date: Date },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::IOError(ref err) => write!(f, "IOError: {}", err),
            Error::ParseIntError(ref err) => write!(f, "ParseIntError: {}", err),
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

static RE_STR: &str = r"^-- (\d{4})-(\d{2})-(\d{2}) ([^ ]+ )?(\d{2}):(\d{2}) -- (.*)$";


impl DayRaw {

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
        let year = year.parse::<i32>()?;
        let month = month.parse::<u32>()?;
        let day = day.parse::<u32>()?;
        let hour = hour.parse::<u32>()?;
        let minute = minute.parse::<u32>()?;
        let (key, sub_keys) = DayRaw::parse_description(desc);
        let start_ts = chrono::Local.ymd(year, month, day).and_hms(hour, minute, 0);

        Ok(EntryRaw{start_ts, key, sub_keys, raw_data: raw_data.to_string()})
    }

    fn check_date(expected_date: Date, found_date: Date, file: &str, line_nr: u32) -> Result<()>
    {
        if expected_date != found_date {
            return Err(Error::UnexpectedDateError{ file: file.to_string(), line_nr, expected_date, found_date });
        }
        Ok(())
    }

    pub fn parse(stream: &mut std::io::BufRead, expected_date: Option<Date>, file: &str) -> Result<DayRaw>
    {
        let mut line_nr = 0u32;
        let date: Option<Date>;
        let (mut non_empty, mut line) = DayRaw::read_line(stream)?;
        while !non_empty {
            let (tmp_non_empty, tmp_line) = DayRaw::read_line(stream)?;
            non_empty = tmp_non_empty;
            line = tmp_line;
        }
        if line == "" {
            if expected_date.is_none() {
                return Err(Error::MissingDateError{file: file.to_string()});
            }
            return Ok(DayRaw{
                date: expected_date.unwrap(),
                entries: Vec::new(), additional_text: String::new()});
        }
        // handle the entries, if there are some
        let entries = DayRaw::parse_entries(&mut line, stream, &expected_date, file, &mut line_nr)?;
        date = if entries.is_empty() {
            expected_date
        } else {
            Some(entries.get(0).unwrap().start_ts.date())
        };

        // the remaining of the file is the description here we merely check that there is no
        // timestamp
        let mut additional_text = String::new();
        match date {
            Some(date) => Ok(DayRaw{date, entries, additional_text}),
            None => Err(Error::MissingDateError{file: file.to_string()}),
        }
    }

    fn parse_entries(
        line: &mut String, stream: &mut std::io::BufRead,
        expected_date: &Option<Date>, file: &str, line_nr: &mut u32)
        -> Result<Vec<EntryRaw>>
    {
        let line_match = DayRaw::parse_entries_line(&line);
        let mut entries = Vec::new();
        if let EntriesLine::Captures(c) = line_match {
            let mut entry_raw = DayRaw::parse_entry(&c[1], &c[2], &c[3], &c[5], &c[6], &c[7], &line)?;
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
            loop {
                let (non_empty, line) = DayRaw::read_line(stream)?;
                if !non_empty {
                    entries.push(entry_raw);
                    break;
                }
                let line_match = DayRaw::parse_entries_line(&line);
                match line_match {
                    EntriesLine::Captures(c) => {
                        entries.push(entry_raw);
                        entry_raw = DayRaw::parse_entry(&c[1], &c[2], &c[3], &c[5], &c[6], &c[7], &line)?;
                        if expected_date != entry_raw.start_ts.date() {
                            return Err(Error::UnexpectedDateError{
                                file: file.to_string(), line_nr: *line_nr,
                                expected_date, found_date: entry_raw.start_ts.date()});
                        }
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


}

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use std::io;
    use chrono;
    use chrono::TimeZone;
    #[test]
    fn test_parse_entries_line()
    {
        let txt: &str = r"

-- 2018-05-04 Mo 12:27 -- Foo Bar Baz
-- 2018-05-04 Mo 12:47 -- Baz

Hier kommt jetzt einfach nur noch geblubber
";
        let txt = txt.as_bytes();
        let mut txt = io::BufReader::new(txt);
        let entries = super::DayRaw::parse(&mut txt, None, "tst-file");
        let expected = super::DayRaw{
            date: chrono::Local.ymd(2018, 4, 4),
            entries: Vec::new(), additional_text: String::new()};
        assert!(entries.is_ok());
        assert_eq!(entries.unwrap(), expected);
    }
}
