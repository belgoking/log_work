pub mod jira;
pub mod required_time;
pub mod util;
pub mod work_day;

extern crate chrono;
//use chrono::TimeZone;

type Date = chrono::NaiveDate;
type DateTime = chrono::NaiveDateTime;
type Time = chrono::NaiveTime;

#[derive(Debug)]
pub enum Error {
    CommandLine(String),
    IO(std::io::Error),
    ParseInt(std::num::ParseIntError),
    InvalidFileName {
        file: std::path::PathBuf,
    },
    ParseDayType {
        file: String,
        line_nr: u32,
    },
    ParseDay,
    ParseTime,
    TimeNotMonotonic {
        file: String,
        line_nr: u32,
    },
    DuplicateDate {
        file: String,
        line_nr: u32,
    },
    EntryAfterSeparator {
        file: String,
        line_nr: u32,
    },
    MissingDate {
        file: String,
    },
    MissingFinalPause {
        file: String,
    },
    UnexpectedDate {
        file: String,
        line_nr: u32,
        expected_date: Date,
        found_date: Date,
    },
}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        match (self, other) {
            (Error::CommandLine(ref s), Error::CommandLine(ref o)) => s == o,
            (Error::IO(_), Error::IO(_)) => true,
            (Error::ParseInt(_), Error::ParseInt(_)) => true,
            (
                Error::InvalidFileName { file: ref s_file },
                Error::InvalidFileName { file: ref o_file },
            ) => s_file == o_file,
            (
                Error::ParseDayType {
                    file: ref s_file,
                    line_nr: s_line_nr,
                },
                Error::ParseDayType {
                    file: ref o_file,
                    line_nr: o_line_nr,
                },
            ) => s_file == o_file && s_line_nr == o_line_nr,
            (Error::ParseDay, &Error::ParseDay) => true,
            (Error::ParseTime, &Error::ParseTime) => true,
            (
                Error::TimeNotMonotonic {
                    file: ref s_file,
                    line_nr: s_line_nr,
                },
                Error::TimeNotMonotonic {
                    file: ref o_file,
                    line_nr: o_line_nr,
                },
            ) => s_file == o_file && s_line_nr == o_line_nr,
            (
                Error::DuplicateDate {
                    file: ref s_file,
                    line_nr: s_line_nr,
                },
                Error::DuplicateDate {
                    file: ref o_file,
                    line_nr: o_line_nr,
                },
            ) => s_file == o_file && s_line_nr == o_line_nr,
            (
                Error::EntryAfterSeparator {
                    file: ref s_file,
                    line_nr: s_line_nr,
                },
                Error::EntryAfterSeparator {
                    file: ref o_file,
                    line_nr: o_line_nr,
                },
            ) => s_file == o_file && s_line_nr == o_line_nr,
            (Error::MissingDate { file: ref s_file }, Error::MissingDate { file: ref o_file }) => {
                s_file == o_file
            }
            (
                Error::MissingFinalPause { file: ref s_file },
                Error::MissingFinalPause { file: ref o_file },
            ) => s_file == o_file,
            (
                Error::UnexpectedDate {
                    file: ref s_file,
                    line_nr: s_line_nr,
                    expected_date: ref s_expected_date,
                    found_date: ref s_found_date,
                },
                Error::UnexpectedDate {
                    file: ref o_file,
                    line_nr: o_line_nr,
                    expected_date: ref o_expected_date,
                    found_date: ref o_found_date,
                },
            ) => {
                s_file == o_file
                    && s_line_nr == o_line_nr
                    && s_expected_date == o_expected_date
                    && s_found_date == o_found_date
            }
            _ => false,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::CommandLine(ref s) => write!(f, "CommandLineError: {}", s),
            Error::IO(ref err) => write!(f, "IOError: {}", err),
            Error::ParseInt(ref err) => write!(f, "ParseIntError: {}", err),
            Error::InvalidFileName { ref file } => {
                write!(f, "InvalidFileNameError: {:?}", file)
            }
            Error::ParseDayType {
                ref file,
                ref line_nr,
            } => write!(f, "ParseDayTypeError: {}:{}", file, line_nr),
            Error::ParseDay => write!(f, "ParseDay"),
            Error::ParseTime => write!(f, "ParseHour"),
            Error::TimeNotMonotonic {
                ref file,
                ref line_nr,
            } => write!(f, "TimeNotMonotonicError: {}:{}", file, line_nr),
            Error::DuplicateDate {
                ref file,
                ref line_nr,
            } => write!(f, "DuplicateDateError: {}:{}", file, line_nr),
            Error::EntryAfterSeparator {
                ref file,
                ref line_nr,
            } => write!(f, "EntryAfterSeparatorError: {}:{}", file, line_nr),
            Error::MissingDate { ref file } => write!(f, "MissingDateError: {}", file),
            Error::MissingFinalPause { ref file } => {
                write!(f, "MissingFinalPauseError: {}", file)
            }
            Error::UnexpectedDate {
                ref file,
                ref line_nr,
                ref expected_date,
                ref found_date,
            } => write!(
                f,
                "UnexpectedDateError: {}:{}: expected={} found={}",
                file, line_nr, expected_date, found_date
            ),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::IO(err)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Error {
        Error::ParseInt(err)
    }
}

type Result<T> = std::result::Result<T, Error>;
