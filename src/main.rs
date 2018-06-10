extern crate chrono;
#[macro_use] extern crate lazy_static;
extern crate regex;
#[macro_use] extern crate structopt;

mod log_work;

use structopt::StructOpt;

/** TODO
 * Return error if day does not end on Pause
 * Unittests for aggregating functions
 * Unittests for erronuous files
 */

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
