extern crate chrono;
extern crate regex;

use self::chrono::Datelike;
use self::util;
use super::*;
use std;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DayType {
    WorkDay,                           // A - Arbeitstag
    JobTravel { description: String }, // D - Dienstreise
    Sick { description: String },      // K - Krank
    WeekEnd,
    Holiday { name: String },                  // F - Feiertag
    Vacation { description: String },          // U - Urlaub
    VacationHalfDay { description: String },   // H - Halber Tag Urlaub
    OvertimeReduction { description: String }, // Ü - Überstundenabbau
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd)]
enum DayTypeClass {
    Work,
    Vacation,
    WeekendAndHolidays,
}

impl DayType {
    fn to_day_type_class(&self) -> DayTypeClass {
        match *self {
            DayType::WorkDay => DayTypeClass::Work,
            DayType::OvertimeReduction { description: _ } => DayTypeClass::Work,
            DayType::WeekEnd => DayTypeClass::WeekendAndHolidays,
            DayType::JobTravel { description: _ } => DayTypeClass::Work,
            DayType::Sick { description: _ } => DayTypeClass::Work,
            DayType::Holiday { name: _ } => DayTypeClass::WeekendAndHolidays,
            DayType::Vacation { description: _ } => DayTypeClass::Vacation,
            DayType::VacationHalfDay { description: _ } => DayTypeClass::Vacation,
        }
    }
}

impl std::fmt::Display for DayType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            DayType::WorkDay => write!(f, "WorkDay"),
            DayType::OvertimeReduction { description: ref s } => {
                write!(f, "OvertimeReduction({})", s)
            }
            DayType::WeekEnd => write!(f, "WeekEnd"),
            DayType::JobTravel { description: ref s } => write!(f, "JobTravel({})", s),
            DayType::Sick { description: ref s } => write!(f, "Sick({})", s),
            DayType::Holiday { name: ref s } => write!(f, "Holiday({})", s),
            DayType::Vacation { description: ref s } => write!(f, "Vacation({})", s),
            DayType::VacationHalfDay { description: ref s } => write!(f, "VacationHalfDay({})", s),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DayTypeEntry {
    date: Date,
    day_type: DayType,
    given_as_range: bool,
    line_nr: u32,
}

fn get_day_type_description(c: &regex::Captures) -> String {
    if c.get(4).is_some() {
        return c[4].to_string();
    }
    // if c[4] is not None c[5] must be there
    c[5].to_string()
}

fn check_day_types(orig: &DayTypeEntry, new_entry: &DayTypeEntry) -> Result<()> {
    if orig.date != new_entry.date {
        return Ok(());
    }
    if orig.day_type == DayType::WorkDay || new_entry.day_type == DayType::WorkDay {
        return Ok(());
    }
    if !orig.given_as_range && orig.day_type.to_day_type_class() != DayTypeClass::WeekendAndHolidays
    {
        match orig.day_type {
            DayType::JobTravel { description: _ } => (),
            _ => {
                return Err(Error::DuplicateDate {
                    file: "".to_string(), /*orig.file.clone()*/
                    line_nr: orig.line_nr,
                });
            }
        };
    }
    if !new_entry.given_as_range
        && new_entry.day_type.to_day_type_class() != DayTypeClass::WeekendAndHolidays
    {
        match new_entry.day_type {
            DayType::JobTravel { description: _ } => (),
            _ => {
                return Err(Error::DuplicateDate {
                    file: "".to_string(), /*new_entry.file.clone()*/
                    line_nr: new_entry.line_nr,
                });
            }
        };
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequiredTime {
    pub date: Date,
    pub day_type: DayType,
    pub required_time: chrono::Duration,
    pub line_nr: u32,
}

impl std::fmt::Display for RequiredTime {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Day: {} Type: {}",
            self.date.format("%F (%a)"),
            self.day_type
        )
    }
}

pub fn consolidate_required_time(
    raw_entries: &Vec<DayTypeEntry>,
    start_date: &Date,
    end_date: &Date,
    duration_of_day: &chrono::Duration,
) -> Result<Vec<RequiredTime>> {
    let mut map: std::collections::BTreeMap<Date, DayTypeEntry> = std::collections::BTreeMap::new();
    for raw_entry in raw_entries {
        let old_entry = map.entry(raw_entry.date);
        match old_entry {
            std::collections::btree_map::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert((*raw_entry).clone());
            }
            std::collections::btree_map::Entry::Occupied(mut occupied_entry) => {
                check_day_types(occupied_entry.get(), raw_entry)?;
                if raw_entry.day_type.to_day_type_class()
                    > occupied_entry.get().day_type.to_day_type_class()
                {
                    *occupied_entry.get_mut() = (*raw_entry).clone();
                }
            }
        }
    }
    let mut ret: Vec<RequiredTime> = Vec::new();
    let mut curr_date = *start_date;
    while curr_date <= *end_date {
        match map.get(&curr_date) {
            Some(day_type_entry) => {
                check_day_types(
                    &DayTypeEntry {
                        date: curr_date,
                        day_type: compute_day_type(&curr_date),
                        given_as_range: true,
                        line_nr: 0,
                    },
                    day_type_entry,
                )?;
                ret.push(RequiredTime {
                    date: curr_date,
                    day_type: day_type_entry.day_type.clone(),
                    required_time: compute_required_time(
                        &curr_date,
                        &day_type_entry.day_type,
                        duration_of_day,
                    ),
                    line_nr: day_type_entry.line_nr,
                });
            }
            None => {
                ret.push(RequiredTime {
                    date: curr_date,
                    day_type: compute_day_type(&curr_date),
                    required_time: compute_simple_required_time(&curr_date, duration_of_day),
                    line_nr: 0,
                });
            }
        }
        curr_date = curr_date.succ_opt().ok_or_else(|| Error::ParseDay)?;
    }
    Ok(ret)
}

fn compute_day_type(date: &Date) -> DayType {
    if date.weekday().num_days_from_monday() <= 4 {
        DayType::WorkDay
    } else {
        DayType::WeekEnd
    }
}

fn compute_simple_required_time(
    date: &Date,
    duration_of_day: &chrono::Duration,
) -> chrono::Duration {
    match compute_day_type(date) {
        DayType::WorkDay => *duration_of_day,
        DayType::WeekEnd => chrono::Duration::hours(0),
        _ => {
            panic!("Error in compute_day_type()");
        }
    }
}

fn compute_required_time(
    date: &Date,
    day_type: &DayType,
    duration_of_day: &chrono::Duration,
) -> chrono::Duration {
    if let DayType::WeekEnd = compute_day_type(date) {
        return chrono::Duration::hours(0);
    };
    match *day_type {
        DayType::JobTravel { description: _ } | DayType::Sick { description: _ } => {
            chrono::Duration::hours(0)
        }
        DayType::Holiday { name: _ } | DayType::Vacation { description: _ } => {
            chrono::Duration::hours(0)
        }
        DayType::VacationHalfDay { description: _ } => (*duration_of_day) / 2,
        DayType::OvertimeReduction { description: _ } => *duration_of_day,
        DayType::WeekEnd | DayType::WorkDay => {
            panic!("illegal DayType: {:?}", *day_type);
        }
    }
}

pub fn parse_required_time_file(file_name: &std::path::PathBuf) -> Result<Vec<DayTypeEntry>> {
    let file = std::fs::File::open(file_name)?;
    let mut fstream = std::io::BufReader::new(file);
    let file_name_str = match file_name.to_str() {
        Some(fi) => fi,
        None => {
            return Err(Error::InvalidFileName {
                file: file_name.clone(),
            })
        }
    };
    let ret = parse_required_time(&mut fstream, file_name_str)?;
    Ok(ret)
}

fn day_type_from_str(s: &str, file_name: &str, line_nr: u32) -> Result<DayType> {
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"^([WKFUHÜ]) +((([^:]*):)|([^ ]*)).*$")
            .expect("Erronuous Regular Expression for holiday type parsing");
    }
    match RE.captures(s) {
        Some(c) => match &c[1] {
            "W" => Ok(DayType::JobTravel {
                description: get_day_type_description(&c),
            }),
            "K" => Ok(DayType::Sick {
                description: get_day_type_description(&c),
            }),
            "F" => Ok(DayType::Holiday {
                name: get_day_type_description(&c),
            }),
            "U" => Ok(DayType::Vacation {
                description: get_day_type_description(&c),
            }),
            "H" => Ok(DayType::VacationHalfDay {
                description: get_day_type_description(&c),
            }),
            "Ü" => Ok(DayType::OvertimeReduction {
                description: get_day_type_description(&c),
            }),
            _ => Err(Error::ParseDayType {
                file: file_name.to_string(),
                line_nr,
            }),
        },
        None => Err(Error::ParseDayType {
            file: file_name.to_string(),
            line_nr,
        }),
    }
}

pub fn parse_required_time(
    stream: &mut dyn std::io::BufRead,
    file_name: &str,
) -> Result<Vec<DayTypeEntry>> {
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(
            r"^(\d{4})-(\d{2})-(\d{2})(--(\d{4})-(\d{2})-(\d{2}))? -- +(.*?) *
?$"
        )
        .expect("Erronuous Regular Expression for holiday parsing");
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
            let (end_date, given_as_range) = match c.get(4) {
                Some(_) => (util::to_date(&c[5], &c[6], &c[7])?, true),
                None => (start_date, false),
            };
            if start_date > end_date {
                return Err(Error::ParseDayType {
                    file: file_name.to_string(),
                    line_nr,
                });
            }
            let day_type = day_type_from_str(&c[8], file_name, line_nr)?;
            let mut curr_day = start_date;
            while curr_day <= end_date {
                ret.push(DayTypeEntry {
                    date: curr_day,
                    day_type: day_type.clone(),
                    given_as_range,
                    line_nr,
                });
                curr_day = curr_day.succ_opt().ok_or_else(|| Error::ParseDay)?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use self::chrono;
    use super::*;
    use std::io;

    #[test]
    fn test_parse_required_time_1() {
        let txt: &str = r"2018-05-04 -- W Mehrere Worte:";
        let expected = Ok(vec![DayTypeEntry {
            date: Date::from_ymd_opt(2018, 5, 4)
                .ok_or_else(|| Error::ParseDay)
                .unwrap(),
            day_type: DayType::JobTravel {
                description: "Mehrere Worte".to_string(),
            },
            given_as_range: false,
            line_nr: 1,
        }]);
        do_test_parse_required_time(txt, expected);
    }

    #[test]
    fn test_parse_required_time_2() {
        let txt: &str = r"2018-05-04--2018-05-05 -- H This is a half day";
        let expected = Ok(vec![
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
                day_type: DayType::VacationHalfDay {
                    description: "This".to_string(),
                },
                given_as_range: true,
                line_nr: 1,
            },
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 5).unwrap(),
                day_type: DayType::VacationHalfDay {
                    description: "This".to_string(),
                },
                given_as_range: true,
                line_nr: 1,
            },
        ]);
        do_test_parse_required_time(txt, expected);
    }

    #[test]
    fn test_parse_required_time_3() {
        let txt: &str = r"2018-05-04--2018-05-05 -- K This is: a sickness day";
        let expected = Ok(vec![
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
                day_type: DayType::Sick {
                    description: "This is".to_string(),
                },
                given_as_range: true,
                line_nr: 1,
            },
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 5).unwrap(),
                day_type: DayType::Sick {
                    description: "This is".to_string(),
                },
                given_as_range: true,
                line_nr: 1,
            },
        ]);
        do_test_parse_required_time(txt, expected);
    }

    #[test]
    fn test_parse_required_time_4() {
        let txt: &str = r"2018-05-04 -- F This is: a holiday
2018-05-07 -- U A vacation day
2018-05-06 -- Ü Brückentag";
        let expected = Ok(vec![
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
                day_type: DayType::Holiday {
                    name: "This is".to_string(),
                },
                given_as_range: false,
                line_nr: 1,
            },
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 7).unwrap(),
                day_type: DayType::Vacation {
                    description: "A".to_string(),
                },
                given_as_range: false,
                line_nr: 2,
            },
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 6).unwrap(),
                day_type: DayType::OvertimeReduction {
                    description: "Brückentag".to_string(),
                },
                given_as_range: false,
                line_nr: 3,
            },
        ]);
        do_test_parse_required_time(txt, expected);
    }

    #[test]
    fn test_parse_required_time_with_foo() {
        let txt: &str = r"foo
2018-05-04 -- F This is: a holiday
bar
2018-05-07 -- U A vacation day
-- 2018-05-06 -- Ü Brückentag
#2018-05-06 -- Ü Brückentag";
        let expected = Ok(vec![
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
                day_type: DayType::Holiday {
                    name: "This is".to_string(),
                },
                given_as_range: false,
                line_nr: 2,
            },
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 7).unwrap(),
                day_type: DayType::Vacation {
                    description: "A".to_string(),
                },
                given_as_range: false,
                line_nr: 4,
            },
        ]);
        do_test_parse_required_time(txt, expected);
    }

    #[test]
    fn test_parse_required_time_with_unknown_type_error() {
        let txt: &str = r"2018-05-04 -- F This is: a holiday
2018-05-07 -- u A half day";
        let expected = Err(Error::ParseDayType {
            file: "tst_file".to_string(),
            line_nr: 2,
        });
        do_test_parse_required_time(txt, expected);
    }

    #[test]
    fn test_parse_required_time_with_invalid_range_error() {
        let txt: &str = r"2018-05-04--2018-04-05 -- F This is: a holiday";
        let expected = Err(Error::ParseDayType {
            file: "tst_file".to_string(),
            line_nr: 1,
        });
        do_test_parse_required_time(txt, expected);
    }

    fn do_test_parse_required_time(txt: &str, expected: Result<Vec<DayTypeEntry>>) {
        let txt = txt.as_bytes();
        let mut txt = io::BufReader::new(txt);
        let parsed_entries = parse_required_time(&mut txt, "tst_file");

        assert_eq!(parsed_entries, expected);
    }

    #[test]
    fn test_consolidate_required_time() {
        let special_required_times = vec![
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
                day_type: DayType::Holiday {
                    name: "ho".to_string(),
                },
                given_as_range: false,
                line_nr: 1,
            },
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 6).unwrap(),
                day_type: DayType::Vacation {
                    description: "A".to_string(),
                },
                given_as_range: true,
                line_nr: 5,
            },
        ];
        let full_day_duration = chrono::Duration::hours(7) + chrono::Duration::minutes(42);
        let result = consolidate_required_time(
            &special_required_times,
            &Date::from_ymd_opt(2018, 5, 3).unwrap(),
            &Date::from_ymd_opt(2018, 5, 6).unwrap(),
            &full_day_duration,
        );
        let expected = Ok(vec![
            RequiredTime {
                date: Date::from_ymd_opt(2018, 5, 3).unwrap(),
                day_type: DayType::WorkDay,
                required_time: full_day_duration,
                line_nr: 0,
            },
            RequiredTime {
                date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
                day_type: DayType::Holiday {
                    name: "ho".to_string(),
                },
                required_time: chrono::Duration::hours(0),
                line_nr: 1,
            },
            RequiredTime {
                date: Date::from_ymd_opt(2018, 5, 5).unwrap(),
                day_type: DayType::WeekEnd,
                required_time: chrono::Duration::hours(0),
                line_nr: 0,
            },
            RequiredTime {
                date: Date::from_ymd_opt(2018, 5, 6).unwrap(),
                day_type: DayType::Vacation {
                    description: "A".to_string(),
                },
                required_time: chrono::Duration::hours(0),
                line_nr: 5,
            },
        ]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_required_time_with_conflict_error_1() {
        let special_required_times = vec![
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
                day_type: DayType::Holiday {
                    name: "ho".to_string(),
                },
                given_as_range: false,
                line_nr: 1,
            },
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
                day_type: DayType::Vacation {
                    description: "A".to_string(),
                },
                given_as_range: false,
                line_nr: 5,
            },
        ];
        let full_day_duration = chrono::Duration::hours(7) + chrono::Duration::minutes(42);
        let result = consolidate_required_time(
            &special_required_times,
            &Date::from_ymd_opt(2018, 5, 3).unwrap(),
            &Date::from_ymd_opt(2018, 5, 5).unwrap(),
            &full_day_duration,
        );
        let expected = Err(Error::DuplicateDate {
            file: "".to_string(),
            line_nr: 5,
        });
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_required_time_with_conflict_error_2() {
        let special_required_times = vec![DayTypeEntry {
            date: Date::from_ymd_opt(2018, 5, 5).unwrap(),
            day_type: DayType::Vacation {
                description: "A".to_string(),
            },
            given_as_range: false,
            line_nr: 5,
        }];
        let full_day_duration = chrono::Duration::hours(7) + chrono::Duration::minutes(42);
        let result = consolidate_required_time(
            &special_required_times,
            &Date::from_ymd_opt(2018, 5, 3).unwrap(),
            &Date::from_ymd_opt(2018, 5, 5).unwrap(),
            &full_day_duration,
        );
        let expected = Err(Error::DuplicateDate {
            file: "".to_string(),
            line_nr: 5,
        });
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_required_time_duplicate_without_conflict() {
        let special_required_times = vec![
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
                day_type: DayType::Holiday {
                    name: "ho".to_string(),
                },
                given_as_range: false,
                line_nr: 1,
            },
            DayTypeEntry {
                date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
                day_type: DayType::Vacation {
                    description: "A".to_string(),
                },
                given_as_range: true,
                line_nr: 5,
            },
        ];
        let full_day_duration = chrono::Duration::hours(7) + chrono::Duration::minutes(42);
        let result = consolidate_required_time(
            &special_required_times,
            &Date::from_ymd_opt(2018, 5, 3).unwrap(),
            &Date::from_ymd_opt(2018, 5, 5).unwrap(),
            &full_day_duration,
        );
        let expected = Ok(vec![
            RequiredTime {
                date: Date::from_ymd_opt(2018, 5, 3).unwrap(),
                day_type: DayType::WorkDay,
                required_time: full_day_duration,
                line_nr: 0,
            },
            RequiredTime {
                date: Date::from_ymd_opt(2018, 5, 4).unwrap(),
                day_type: DayType::Holiday {
                    name: "ho".to_string(),
                },
                required_time: chrono::Duration::hours(0),
                line_nr: 1,
            },
            RequiredTime {
                date: Date::from_ymd_opt(2018, 5, 5).unwrap(),
                day_type: DayType::WeekEnd,
                required_time: chrono::Duration::hours(0),
                line_nr: 0,
            },
        ]);
        assert_eq!(result, expected);
    }
}
