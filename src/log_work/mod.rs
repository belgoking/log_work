pub mod required_time;
pub mod work_day;
mod util;

extern crate chrono;
//use chrono::TimeZone;

use std;

type Date = chrono::Date<chrono::Local>;
type DateTime = chrono::DateTime<chrono::Local>;
type Time = chrono::NaiveTime;

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    ParseIntError(std::num::ParseIntError),
    InvalidFileNameError{file: std::path::PathBuf},
    ParseDayTypeError{file: String, line_nr: u32},
    TimeNotMonotonicError{file: String, line_nr: u32},
    DuplicateDateError{file: String, line_nr: u32},
    MissingDateError{file: String},
    UnexpectedDateError{file: String, line_nr: u32,
        expected_date: Date,
        found_date: Date},
}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        use self::Error;
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
            (&Error::DuplicateDateError{file: ref s_file, line_nr: s_line_nr},
             &Error::DuplicateDateError{file: ref o_file, line_nr: o_line_nr}) =>
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
            Error::DuplicateDateError{ref file, ref line_nr} =>
                write!(f, "DuplicateDateError: {}:{}", file, line_nr),
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


