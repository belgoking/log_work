extern crate chrono;
#[macro_use] extern crate lazy_static;
extern crate regex;
#[macro_use] extern crate structopt;

mod log_work;

use structopt::StructOpt;

use std::io::BufRead;

/** TODO
 * Return error if day does not end on Pause
 * Unittests for aggregating functions
 * Unittests for erronuous files
 */

fn parse_duration(s: &str) -> Result<chrono::Duration, log_work::Error> {
    let re = regex::Regex::new(r"^((\d+)h)? ?((\d+)m)?$").expect("broken regular expression");
    match re.captures(s) {
        Some(c) => {
            let h = c.get(2).map_or("0", |m| m.as_str());
            let h = h.parse::<i64>()?;
            let m = c.get(4).map_or("0", |m| m.as_str());
            let m = m.parse::<i64>()?;
            return Ok(chrono::Duration::hours(h) + chrono::Duration::minutes(m));
        },
        None => return Err(log_work::Error::CommandLineError("Command line argument did not have the form '<hours>h <minutes>m'".to_string()))
    }
}

#[derive(Debug, StructOpt)]
#[structopt(about="Read .work-files and give summaries of worked time.")]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct Opt {
    /// A file containing holidays and vacations
    #[structopt(short="H", long="holidays", parse(from_os_str))]
    holidays: Option<std::path::PathBuf>,

    /// Write debugging output
    #[structopt(short="d", long="debug")]
    debug: bool,

    /// Print more details
    #[structopt(short="v", long="verbose")]
    verbose: bool,

    /// Don't abort on some errors. Especially don't abort if a day does not end on a pause.
    #[structopt(short="l", long="lenient")]
    be_lenient: bool,

    /// The duration of a work-day matching the expressoin '(\d+h)? ?(\d+m)?' with the first part
    /// denominating the hours and the second part the minutes.
    #[structopt(short="u", long="duration_of_day", parse(try_from_str="parse_duration"))]
    duration_of_day: Option<chrono::Duration>,

    /// The .work-files
    #[structopt(parse(from_os_str))]
    files: Vec<std::path::PathBuf>,
}

fn main() {
    let mut opt_from_file = if let Ok(home) = std::env::var("HOME") {
            let rc_file = home + "/.log_work.rc";
            if let Ok(f) = std::fs::File::open(rc_file) {
                let mut lines: Vec<String> = std::io::BufReader::new(f).lines().map(|e| e.unwrap()).collect();
                lines.insert(0, "DUMMY".to_string()); // normally the first element holds the program name
                Opt::from_iter(lines.iter())
            } else {
                Opt{holidays: None, debug: false, verbose: false, be_lenient: false, duration_of_day: None, files: Vec::new()}
            }
        } else {
            Opt{holidays: None, debug: false, verbose: false, be_lenient: false, duration_of_day: None, files: Vec::new()}
        };
    let opt_from_args = Opt::from_args();

    if opt_from_args.debug || opt_from_file.debug {
        println!("file={:?} cmd={:?}", opt_from_file, opt_from_args);
    }
    let mut files = opt_from_args.files;
    files.append(&mut opt_from_file.files);
    let opt = Opt{
            holidays: if let Some(h) = opt_from_args.holidays { Some(h) } else { opt_from_file.holidays },
            debug: opt_from_args.debug || opt_from_file.debug,
            verbose: opt_from_args.verbose || opt_from_file.verbose,
            be_lenient: opt_from_args.be_lenient || opt_from_file.be_lenient,
            duration_of_day: if let Some(d) = opt_from_args.duration_of_day { Some(d) } else { opt_from_file.duration_of_day },
            files: files
        };

    if opt.debug {
        println!("opt={:?}", opt);
    }
    let work_days_raw = log_work::work_day::Days::parse_work_files(opt.files, opt.be_lenient);
    if work_days_raw.is_empty() {
        println!("No days given, aborting!");
        return;
    }

    let mut has_error = false;
    let mut work_day_by_date = std::collections::BTreeMap::new();
    for ref day_raw in &work_days_raw {
        if opt.debug {
            println!("Day: {:?}", day_raw);
        }
        if let Err(e) = day_raw {
            println!("ERROR: {:?}", e);
            has_error = true;
            continue;
        }
        let day_raw = day_raw.as_ref().unwrap();
        if let Some(ref e) = work_day_by_date.insert(day_raw.date.clone(), (*day_raw).clone()) {
            println!("Duplicate day: {:?}", e);
            has_error = true;
        }
    }
    if has_error {
        println!("Abort because of errors");
        return;
    };
    let first_date = &work_days_raw[0].as_ref().unwrap().date;
    let (min_day, max_day) = work_days_raw.iter().fold((*first_date, *first_date),
                                                |(min, max), ref day| {
                                                    let date = &day.as_ref().unwrap().date;
                                                    (std::cmp::min(min, *date), std::cmp::max(max, *date))
                                                });
    if opt.debug {
        println!("min={} max={}", min_day.format("%F"), max_day.format("%F"));
    }
    let duration_of_day = if let Some(d) = opt.duration_of_day { d } else { chrono::Duration::hours(8) };
    let required_time =
        match opt.holidays {
            Some(fp) => {
                let required_time = log_work::required_time::parse_required_time_file(&fp).expect("Error parsing required time file");
                log_work::required_time::consolidate_required_time(&required_time, &min_day, &max_day, &duration_of_day)
                   .expect("Failed to consolidate required times")
            },
            None => {
                let empty = Vec::new();
                log_work::required_time::consolidate_required_time(&empty, &min_day, &max_day, &duration_of_day)
                    .expect("Failed to consolidate required times")
            },
        };
    if opt.debug {
        println!("Required-times: {:?}", required_time);
    }

    let days = log_work::work_day::Days::join_work_and_requirement(&work_day_by_date, &required_time, &duration_of_day);
    if opt.debug {
        for ref day in &days.days {
            println!("Required-times for {:?}: {:?}", day.required_time.date, day);
        }
    }

    let mut summary = log_work::work_day::Summary::new();
    let mut sum_required = chrono::Duration::hours(0);
    for ref day in &days.days {
        println!("{}", log_work::work_day::DaySummary{day: &day, verbose: opt.verbose});
        log_work::work_day::WorkDay::merge_summaries_right_into_left(&mut summary, &day.work_day.compute_summary());
        sum_required = sum_required + day.required_time.required_time;
    }
    println!("= Summary for all days: Required: {}",
             log_work::util::WorkDuration{duration: sum_required, duration_of_day});
    let mut sum = chrono::Duration::hours(0);
    for (key, duration) in summary.iter() {
        println!("{:20}: {}", key, log_work::util::WorkDuration{ duration_of_day, duration: *duration });
        if key != "Pause" {
            sum = sum + *duration;
        }
    }
    println!("{:20}: {}", " == Total ==", log_work::util::WorkDuration{ duration_of_day, duration: sum });
}
