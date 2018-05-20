mod log_work;

#[macro_use] extern crate lazy_static;
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

    let required_time =
        match opt.holidays {
            Some(fp) => {
                Some(log_work::required_time::parse_required_time_file(&fp))
            },
            None => None,
        };
    println!("Required-times: {:?}", required_time);

    for day_raw in log_work::work_day::Days::parse_work_files(opt.files) {
        println!("Day: {:?}", day_raw);
    }
}
