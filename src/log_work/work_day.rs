use self::util;
use super::*;
use std;

#[derive(Clone, Debug, Eq, PartialEq)]
struct EntryRaw {
    pub start_ts: DateTime,
    pub key: String,
    pub sub_keys: Vec<String>,
    pub raw_data: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    pub start_ts: Time,
    pub duration: chrono::Duration,
    pub key: String,
    pub sub_keys: Vec<String>,
    pub raw_data: String,
}

impl Entry {
    fn from(entries: Vec<EntryRaw>) -> Vec<Entry> {
        let mut ret = Vec::new();
        if entries.is_empty() {
            return ret;
        }
        ret.reserve_exact(entries.len());
        let mut old_entry: Option<EntryRaw> = None;
        for new_entry in entries {
            old_entry = match old_entry {
                Option::Some(old_entry) => {
                    let duration = new_entry.start_ts.time() - old_entry.start_ts.time();
                    ret.push(Entry {
                        start_ts: old_entry.start_ts.time(),
                        duration,
                        key: old_entry.key,
                        sub_keys: old_entry.sub_keys,
                        raw_data: old_entry.raw_data,
                    });
                    Some(new_entry)
                }
                Option::None => Some(new_entry),
            };
        }
        let old_entry = old_entry.unwrap();
        ret.push(Entry {
            start_ts: old_entry.start_ts.time(),
            duration: chrono::Duration::minutes(0),
            key: old_entry.key,
            sub_keys: old_entry.sub_keys,
            raw_data: old_entry.raw_data,
        });
        ret
    }
}

#[derive(Debug)]
enum EntriesLine<'a> {
    Captures(regex::Captures<'a>),
    Line,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkDay {
    pub date: Date,
    pub entries: Vec<Entry>,
    pub additional_text: String,
}

impl WorkDay {
    fn read_line(stream: &mut dyn std::io::BufRead) -> Result<(bool, String)> {
        let mut line = String::new();
        let num_bytes = stream.read_line(&mut line)?;
        Ok(((num_bytes != 0 && line != "\n" && line.chars().nth(0).unwrap() != '#'), line))
    }

    fn parse_entries_line(line: &str) -> EntriesLine {
        lazy_static::lazy_static! {
            static ref RE: regex::Regex = regex::Regex::new(
                "^-- (\\d{4})-(\\d{2})-(\\d{2}) ([^ ]+ )?(\\d{2}):(\\d{2}) -- (.*)\n?$"
            )
            .expect("Erronuous Regular Expression");
        }
        let cap = RE.captures(line);
        match cap {
            Some(c) => EntriesLine::Captures(c),
            None => EntriesLine::Line,
        }
    }

    fn parse_description(description: &str) -> (String, Vec<String>) {
        let description = description.trim_start();
        if description.is_empty() {
            return (String::new(), Vec::new());
        }
        let mut iter = description
            .split(|c| c == ' ' || c == ':')
            .filter(|x| !x.is_empty());
        (
            iter.next().unwrap_or("").to_string(),
            iter.map(|x| x.to_owned()).collect(),
        )
    }

    fn parse_entry(
        year: &str,
        month: &str,
        day: &str,
        hour: &str,
        minute: &str,
        desc: &str,
        raw_data: &str,
    ) -> Result<EntryRaw> {
        let hour = hour.parse::<u32>()?;
        let minute = minute.parse::<u32>()?;
        let (key, sub_keys) = WorkDay::parse_description(desc);
        let date = util::to_date(year, month, day)?;
        let start_ts = date
            .and_hms_opt(hour, minute, 0)
            .ok_or_else(|| Error::ParseTime)?;

        Ok(EntryRaw {
            start_ts,
            key,
            sub_keys,
            raw_data: raw_data.to_string(),
        })
    }

    pub fn parse(
        stream: &mut dyn std::io::BufRead,
        expected_date: Option<Date>,
        be_lenient: bool,
        file: &str,
    ) -> Result<WorkDay> {
        let mut line_nr = 0u32;
        let (mut non_empty, mut line) = WorkDay::read_line(stream)?;
        while !non_empty {
            line_nr += 1;
            let (tmp_non_empty, tmp_line) = WorkDay::read_line(stream)?;
            non_empty = tmp_non_empty;
            line = tmp_line;
        }
        if line.is_empty() {
            if expected_date.is_none() {
                return Err(Error::MissingDate {
                    file: file.to_string(),
                });
            }
            return Ok(WorkDay {
                date: expected_date.unwrap(),
                entries: Vec::new(),
                additional_text: String::new(),
            });
        }
        // handle the entries, if there are some
        let entries = WorkDay::parse_entries(line, stream, &expected_date, file, &mut line_nr)?;
        let date = if entries.is_empty() {
            expected_date
        } else {
            Some(entries.first().unwrap().start_ts.date())
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
            if let EntriesLine::Captures(_) = WorkDay::parse_entries_line(&line) {
                return Err(Error::EntryAfterSeparator {
                    file: file.to_string(),
                    line_nr,
                });
            }
            additional_text.push_str(&line[..]);
            let (_, tmp_line) = WorkDay::read_line(stream)?;
            line = tmp_line;
        }
        if !entries.is_empty() && &entries.last().unwrap().key != "Pause" {
            if be_lenient {
                // TODO: log a warning using a logger
                println!(
                    "WARNING: Missing 'Pause' as last entry for the day for file '{}'!",
                    file
                );
            } else {
                return Err(Error::MissingFinalPause {
                    file: file.to_string(),
                });
            }
        }
        match date {
            Some(date) => {
                let entries = Entry::from(entries);
                Ok(WorkDay {
                    date,
                    entries,
                    additional_text,
                })
            }
            None => Err(Error::MissingDate {
                file: file.to_string(),
            }),
        }
    }

    fn parse_entries(
        line: String,
        stream: &mut dyn std::io::BufRead,
        expected_date: &Option<Date>,
        file: &str,
        line_nr: &mut u32,
    ) -> Result<Vec<EntryRaw>> {
        let line_match = WorkDay::parse_entries_line(&line);
        let mut entries = Vec::new();
        if let EntriesLine::Captures(c) = line_match {
            let mut entry_raw =
                WorkDay::parse_entry(&c[1], &c[2], &c[3], &c[5], &c[6], &c[7], &line)?;
            *line_nr += 1;
            let expected_date = match *expected_date {
                None => entry_raw.start_ts.date(),
                Some(expected_date) => {
                    let found_date = entry_raw.start_ts.date();
                    if expected_date != found_date {
                        return Err(Error::UnexpectedDate {
                            file: file.to_string(),
                            line_nr: *line_nr,
                            expected_date,
                            found_date,
                        });
                    }
                    found_date
                }
            };
            let mut last_ts = entry_raw.start_ts;
            loop {
                let (non_empty, line) = WorkDay::read_line(stream)?;
                if !non_empty {
                    if !line.is_empty() {
                        *line_nr += 1;
                    }
                    entries.push(entry_raw);
                    break;
                }
                let line_match = WorkDay::parse_entries_line(&line);
                *line_nr += 1;
                match line_match {
                    EntriesLine::Captures(c) => {
                        entries.push(entry_raw);
                        entry_raw =
                            WorkDay::parse_entry(&c[1], &c[2], &c[3], &c[5], &c[6], &c[7], &line)?;
                        if expected_date != entry_raw.start_ts.date() {
                            return Err(Error::UnexpectedDate {
                                file: file.to_string(),
                                line_nr: *line_nr,
                                expected_date,
                                found_date: entry_raw.start_ts.date(),
                            });
                        }
                        if last_ts > entry_raw.start_ts {
                            return Err(Error::TimeNotMonotonic {
                                file: file.to_string(),
                                line_nr: *line_nr,
                            });
                        }
                        last_ts = entry_raw.start_ts;
                    }
                    EntriesLine::Line => {
                        entry_raw = EntryRaw {
                            raw_data: entry_raw.raw_data + &line,
                            ..entry_raw
                        };
                    }
                }
            }
        };
        Ok(entries)
    }

    pub fn parse_file(file_name: &std::path::PathBuf, be_lenient: bool) -> Result<WorkDay> {
        lazy_static::lazy_static! {
            static ref RE: regex::Regex =
                regex::Regex::new(r"(^|/)(\d{4})(\d{2})(\d{2})(_.*)\.work$")
                    .expect("Erronuous Regular Expression");
        }

        let file_name_str = match file_name.to_str() {
            Some(fi) => fi,
            None => {
                return Err(Error::InvalidFileName {
                    file: file_name.clone(),
                })
            }
        };
        let expected_date: Option<Date> = match RE.captures(file_name_str) {
            Some(c) => {
                let y = c[2].parse::<i32>()?;
                let m = c[3].parse::<u32>()?;
                let d = c[4].parse::<u32>()?;
                Date::from_ymd_opt(y, m, d)
            }
            None => None,
        };
        let file = std::fs::File::open(file_name)?;
        let mut fstream = std::io::BufReader::new(file);
        WorkDay::parse(&mut fstream, expected_date, be_lenient, file_name_str)
    }

    pub fn compute_summary(&self) -> Summary {
        let mut ret = Summary::new();
        for entry in &self.entries {
            ret.entry(entry.key.clone())
                .and_modify(|e| *e += entry.duration)
                .or_insert(entry.duration);
        }
        ret
    }

    pub fn merge_summaries_right_into_left(left: &mut Summary, right: &Summary) {
        for (k, v) in right.iter() {
            left.entry(k.to_string())
                .and_modify(|e| *e += *v)
                .or_insert(*v);
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Day {
    pub duration_of_day: chrono::Duration,
    pub required_time: required_time::RequiredTime,
    pub work_day: WorkDay,
}

pub type Summary = std::collections::BTreeMap<String, chrono::Duration>;

pub struct DaySummary<'a> {
    pub day: &'a Day,
    pub verbose: bool,
}

impl<'a> std::fmt::Display for DaySummary<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let duration_of_day = self.day.duration_of_day;
        if self.verbose {
            for entry in &self.day.work_day.entries {
                write!(
                    f,
                    "{} {:>15} {}",
                    &entry.raw_data[0..25],
                    util::WorkDuration {
                        duration_of_day,
                        duration: entry.duration
                    },
                    &entry.raw_data[25..]
                )?;
            }
        }
        writeln!(f, "= {}", self.day.required_time)?;
        let mut sum = chrono::Duration::hours(0);
        for (key, duration) in self.day.work_day.compute_summary().iter() {
            writeln!(
                f,
                "{:20}: {:>19}",
                key,
                util::WorkDuration {
                    duration_of_day,
                    duration: *duration
                }
            )?;
            if key != "Pause" {
                sum += *duration;
            }
        }
        writeln!(
            f,
            "{:20}: {:>19}",
            " == Required ==",
            util::WorkDuration {
                duration_of_day,
                duration: self.day.required_time.required_time
            }
        )?;
        writeln!(
            f,
            "{:20}: {:>19}",
            " == Total ==",
            util::WorkDuration {
                duration_of_day,
                duration: sum
            }
        )?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Days {
    pub days: Vec<Day>,
}

impl Days {
    pub fn parse_work_files(
        mut files: Vec<std::path::PathBuf>,
        be_lenient: bool,
    ) -> Vec<Result<WorkDay>> {
        files.sort();
        let mut ret: Vec<Result<WorkDay>> = Vec::new();
        ret.reserve_exact(files.len());
        for ref file in files {
            ret.push(WorkDay::parse_file(file, be_lenient));
        }
        ret
    }

    pub fn join_work_and_requirement(
        work_days: &std::collections::BTreeMap<Date, WorkDay>,
        required_times: &Vec<required_time::RequiredTime>,
        duration_of_day: &chrono::Duration,
    ) -> Days {
        let mut days = Vec::new();
        days.reserve_exact(required_times.len());
        for required_time in required_times {
            let work_day: WorkDay = match work_days.get(&required_time.date) {
                Some(day) => (*day).clone(),
                None => WorkDay {
                    date: required_time.date,
                    entries: Vec::new(),
                    additional_text: "".to_string(),
                },
            };

            days.push(Day {
                duration_of_day: *duration_of_day,
                required_time: (*required_time).clone(),
                work_day,
            });
        }

        Days { days }
    }

    //    pub fn load(mut files: Vec<std::path::PathBuf>, _special_dates_file: Option<String>) -> Days
    //    {
    //        // read work_files
    //        // read special_dates
    //        // merge both
    //        // throw an exception on duplicate days
    //    }
}

#[cfg(test)]
mod tests {
    use self::chrono;
    use super::*;
    use std::io;

    #[test]
    fn test_parse_error_wrong_day_1() {
        let txt: &str = r"-- 2018-05-04 Mo 12:27 -- Foo Bar Baz";
        let txt = txt.as_bytes();
        let mut txt = io::BufReader::new(txt);
        let expected_date = Date::from_ymd_opt(2018, 5, 3).unwrap();
        let entries = WorkDay::parse(&mut txt, Some(expected_date), false, "tst_file");
        let expected_error = Err(Error::UnexpectedDate {
            file: "tst_file".to_string(),
            line_nr: 1,
            expected_date,
            found_date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
        });
        assert_eq!(expected_error, entries);
    }

    #[test]
    fn test_parse_error_wrong_day_2() {
        let txt: &str = r"

-- 2018-05-04 Mo 12:27 -- Foo Bar Baz";
        let txt = txt.as_bytes();
        let mut txt = io::BufReader::new(txt);
        let expected_date = Date::from_ymd_opt(2018, 5, 3).unwrap();
        let entries = WorkDay::parse(&mut txt, Some(expected_date), false, "tst_file");
        let expected_error = Err(Error::UnexpectedDate {
            file: "tst_file".to_string(),
            line_nr: 3,
            expected_date,
            found_date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
        });
        assert_eq!(expected_error, entries);
    }

    #[test]
    fn test_parse_error_wrong_day_3() {
        let txt: &str = r"
-- 2018-05-03 Mo 12:27 -- Foo Bar Baz
-- 2018-05-04 Mo 12:27 -- Foo Bar Baz";
        let txt = txt.as_bytes();
        let mut txt = io::BufReader::new(txt);
        let expected_date = Date::from_ymd_opt(2018, 5, 3).unwrap();
        let entries = WorkDay::parse(&mut txt, Some(expected_date), false, "tst_file");
        let expected_error = Err(Error::UnexpectedDate {
            file: "tst_file".to_string(),
            line_nr: 3,
            expected_date,
            found_date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
        });
        assert_eq!(expected_error, entries);
    }

    #[test]
    fn test_parse_error_time_non_monotonic() {
        let txt: &str = r"-- 2018-05-04 Mo 12:27 -- Foo Bar Baz
-- 2018-05-04 Mo 12:26 -- Foo Bar Baz";
        let mut txt = io::BufReader::new(txt.as_bytes());
        let entries = WorkDay::parse(&mut txt, None, false, "tst_file");
        let expected_error = Err(Error::TimeNotMonotonic {
            file: "tst_file".to_string(),
            line_nr: 2,
        });
        assert_eq!(expected_error, entries);
    }

    #[test]
    fn test_parse_error_entry_after_separator() {
        let txt: &str = r"-- 2018-05-04 Mo 12:27 -- Foo
-- 2018-05-04 Mo 12:29 -- Bar

-- 2018-05-04 Mo 12:39 -- Baz";
        let mut txt = io::BufReader::new(txt.as_bytes());
        let entries = WorkDay::parse(&mut txt, None, false, "tst_file");
        let expected_error = Err(Error::EntryAfterSeparator {
            file: "tst_file".to_string(),
            line_nr: 4,
        });
        assert_eq!(expected_error, entries);
    }

    #[test]
    fn test_parse_error_missing_final_pause() {
        let txt: &str = r"-- 2018-05-04 Mo 12:27 -- Foo
-- 2018-05-04 Mo 12:29 -- Bar
-- 2018-05-04 Mo 12:39 -- Baz";
        let mut txt = io::BufReader::new(txt.as_bytes());
        let entries = WorkDay::parse(&mut txt, None, false, "tst_file");
        let expected_error = Err(Error::MissingFinalPause {
            file: "tst_file".to_string(),
        });
        assert_eq!(expected_error, entries);
    }

    #[test]
    fn test_parse_missing_final_pause_in_lenient_mode() {
        let txt: &str = r"-- 2018-05-04 Mo 12:27 -- Foo
-- 2018-05-04 Mo 12:29 -- Bar
-- 2018-05-04 Mo 12:39 -- Baz";
        let mut txt = io::BufReader::new(txt.as_bytes());
        let parsed_entries = WorkDay::parse(&mut txt, None, true, "tst_file");
        assert!(parsed_entries.is_ok());

        let expected_entries = vec![
            Entry {
                start_ts: Time::from_hms_opt(12, 27, 0).unwrap(),
                duration: chrono::Duration::minutes(2),
                key: "Foo".to_string(),
                sub_keys: Vec::new(),
                raw_data: "-- 2018-05-04 Mo 12:27 -- Foo\n".to_string(),
            },
            Entry {
                start_ts: Time::from_hms_opt(12, 29, 0).unwrap(),
                duration: chrono::Duration::minutes(10),
                key: "Bar".to_string(),
                sub_keys: Vec::new(),
                raw_data: "-- 2018-05-04 Mo 12:29 -- Bar\n".to_string(),
            },
            Entry {
                start_ts: Time::from_hms_opt(12, 39, 0).unwrap(),
                duration: chrono::Duration::minutes(0),
                key: "Baz".to_string(),
                sub_keys: Vec::new(),
                raw_data: "-- 2018-05-04 Mo 12:39 -- Baz".to_string(),
            },
        ];
        let expected = WorkDay {
            date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
            entries: expected_entries,
            additional_text: String::new(),
        };
        assert_eq!(parsed_entries.unwrap(), expected);
    }

    #[test]
    fn test_parse_entries_line_with_empty_lines() {
        let txt: &str = r"-- 2018-05-04 Mo 12:27 -- Foo
Bar Baz
-- 2018-05-04 Mo 12:47 -- Bam
-- 2018-05-04 Mo 13:48 -- Pause Blah

Hier kommt jetzt einfach nur noch geblubber
";
        let txt = txt.as_bytes();
        let mut txt = io::BufReader::new(txt);
        let parsed_entries = WorkDay::parse(&mut txt, None, false, "tst_file");
        assert!(parsed_entries.is_ok());

        let expected_entries = vec![
            Entry {
                start_ts: Time::from_hms_opt(12, 27, 0).unwrap(),
                duration: chrono::Duration::minutes(20),
                key: "Foo".to_string(),
                sub_keys: Vec::new(),
                raw_data: "-- 2018-05-04 Mo 12:27 -- Foo\nBar Baz\n".to_string(),
            },
            Entry {
                start_ts: Time::from_hms_opt(12, 47, 0).unwrap(),
                duration: chrono::Duration::minutes(61),
                key: "Bam".to_string(),
                sub_keys: Vec::new(),
                raw_data: "-- 2018-05-04 Mo 12:47 -- Bam\n".to_string(),
            },
            Entry {
                start_ts: Time::from_hms_opt(13, 48, 0)
                    .ok_or_else(|| Error::ParseTime)
                    .unwrap(),
                duration: chrono::Duration::minutes(0),
                key: "Pause".to_string(),
                sub_keys: vec!["Blah".to_string()],
                raw_data: "-- 2018-05-04 Mo 13:48 -- Pause Blah\n".to_string(),
            },
        ];
        let expected = WorkDay {
            date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
            entries: expected_entries,
            additional_text: "Hier kommt jetzt einfach nur noch geblubber\n".to_string(),
        };
        assert_eq!(parsed_entries.unwrap(), expected);
    }
}
