extern crate chrono;
#[macro_use] extern crate lazy_static;
extern crate regex;
#[macro_use] extern crate structopt;

mod log_work;

use structopt::StructOpt;

/** TODO
 * Collect sub-keys
 * Collect and sum up the entries of one day
 * Create aggregate of several days
 * Collect and sum up the entries of several days
 * Handle flexible required durations per day
 * Handle vacations/half-day-vacations/sickness/holidays/conferences
 */

#[derive(Debug, StructOpt)]
#[structopt(about="Read .work-files and give summaries of worked time.")]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct Opt {
    /// A file containing holidays and vacations
    #[structopt(short="H", long="holidays", parse(from_os_str))]
    holidays: Option<std::path::PathBuf>,

    /// Write debugging output
    #[structopt(short="v", long="verbose")]
    verbose: bool,

    /// The .work-files
    #[structopt(parse(from_os_str))]
    files: Vec<std::path::PathBuf>,
}

fn main() {
    let opt = Opt::from_args();


    let work_days_raw = log_work::work_day::Days::parse_work_files(opt.files);
    if work_days_raw.is_empty() {
        println!("No days given, aborting!");
        return;
    }

    let mut has_error = false;
    let mut work_day_by_date = std::collections::BTreeMap::new();
    for ref day_raw in &work_days_raw {
        if opt.verbose {
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
    println!("min={} max={}", min_day, max_day);
    let duration_of_day = chrono::Duration::hours(7) + chrono::Duration::minutes(42);
    let required_time =
        match opt.holidays {
            Some(fp) => {
                let required_time = log_work::required_time::parse_required_time_file(&fp).expect("Error parsing required time file");
                log_work::required_time::consolidate_required_time(&required_time, &min_day, &max_day, &duration_of_day)
                    .expect("Failed to consolidate required times")
            },
            None => {
                let empty = Vec::new();
//                let empty = Vec<log_work::required_time::DayTypeEntry>::new();
                log_work::required_time::consolidate_required_time(&empty, &min_day, &max_day, &duration_of_day)
                    .expect("Failed to consolidate required times")
            },
        };
    if opt.verbose {
        println!("Required-times: {:?}", required_time);
    }

    let days = log_work::work_day::Days::join_work_and_requirement(&work_day_by_date, &required_time);
    if opt.verbose {
        for ref day in &days.days {
            println!("Required-times for {:?}: {:?}", day.required_time.date, day);
        }
    }

    let mut summary = log_work::work_day::Summary::new();
    for ref day in &days.days {
        let tmp_summary = day.work_day.compute_summary();
        println!("Summary for day={:?}: {:?}", day.get_date(), tmp_summary);
        summary = log_work::work_day::WorkDay::merge_summaries(summary, &tmp_summary);
    }
    println!("Summary for all days: {:?}", summary);
}
