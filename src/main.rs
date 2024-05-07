mod log_work;

use structopt::StructOpt;

use std::io::BufRead;

lazy_static::lazy_static! {
static ref APP_INFO: directories::ProjectDirs =
    directories::ProjectDirs::from("de", "belgoking", "log_work").unwrap();
}

/** TODO
 * Unittests for aggregating functions
 */

fn parse_duration(s: &str) -> Result<chrono::Duration, log_work::Error> {
    let re = regex::Regex::new(r"^((\d+)h)? ?((\d+)m)?$").expect("broken regular expression");
    match re.captures(s) {
        Some(c) => {
            let h = c.get(2).map_or("0", |m| m.as_str());
            let h = h.parse::<i64>()?;
            let m = c.get(4).map_or("0", |m| m.as_str());
            let m = m.parse::<i64>()?;
            Ok(chrono::Duration::hours(h) + chrono::Duration::minutes(m))
        }
        None => Err(log_work::Error::CommandLine(
            "Command line argument did not have the form '<hours>h <minutes>m'".to_string(),
        )),
    }
}

#[derive(Debug, structopt::StructOpt, Default)]
#[structopt(about = r"Read .work-files and give summaries of worked time.

The format of the .work-files is:
-- yyyy-mm-dd DD HH:MM -- Key1: description1
-- yyyy-mm-dd DD HH:MM -- Key2: description2

Some other text separated by the entries by an empty line.

The format of the holidays-file is:
yyyy-mm-dd -- [WKFUHÜ] description or
yyyy-mm-dd--yyyy-mm-dd -- [WKFUHÜ] description

lines that don't match any of the initial date-patterns are ignored.
The meaning of the keys is:
W - (Weiterbildung) Job Education/Job Travel. These days have an expected
    logged time of 0, so you should not use it, if you plan on logging your
    time for that job travel.
K - (Krank) Sick. Expected logged time is 0.
F - (Feiertag) Holiday. Expected logged time is 0.
U - (Urlaub) Vacation. Expected logged time is 0. This will be added to the
    vacation count.
H - (Halber Tag Urlaub) Half day vacation. Expected logged time is 1/2 of a
    day.
Ü - (Überstundenabbau) Reduction of overtime. Expected logged time is 1 day.
    This is just a marker, such that no warning regarding a missing day is
    generated.
")]
#[structopt(setting = structopt::clap::AppSettings::ColoredHelp)]
struct Opt {
    /// A file containing holidays and vacations
    #[structopt(short = "H", long = "holidays", parse(from_os_str))]
    holidays: Option<std::path::PathBuf>,

    /// Write debugging output
    #[structopt(short = "d", long = "debug")]
    debug: bool,

    /// Print more details
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,

    /// Don't abort on some errors. Especially don't abort if a day does not end on a pause.
    #[structopt(short = "l", long = "lenient")]
    be_lenient: bool,

    /// The duration of a work-day matching the expressoin '(\d+h)? ?(\d+m)?' with the first part
    /// denominating the hours and the second part the minutes.
    #[structopt(
        short = "u",
        long = "duration_of_day",
        parse(try_from_str = parse_duration)
    )]
    duration_of_day: Option<chrono::Duration>,

    /// Timezone in the format `Europe/Berlin` (usually this is not needed. However, Jira requires
    /// timezones for time logging)
    #[structopt(short = "z")]
    timezone: Option<chrono_tz::Tz>,

    /// Log the times of the days to the configured JIRA server
    #[structopt(long = "log_to_jira")]
    log_to_jira: bool,

    /// The base URL of the JIRA server (e.g. 'https://jira.example.com/jira')
    #[structopt(long = "jira_base_url")]
    jira_base_url: Option<String>,

    /// The username of the JIRA user
    #[structopt(long = "jira_username")]
    jira_username: Option<String>,

    /// The password of the JIRA user
    #[structopt(long = "jira_password")]
    jira_password: Option<String>,

    /// The .work-files
    #[structopt(parse(from_os_str))]
    files: Vec<std::path::PathBuf>,
}

fn first_available<T>(opt1: Option<T>, opt2: Option<T>) -> Option<T> {
    match opt1 {
        Some(v) => Some(v),
        None => opt2,
    }
}

fn main() {
    let mut opt_from_file = {
        let mut rc_file = APP_INFO.config_dir().to_path_buf();
        rc_file.push("log_work.rc");
        println!("Application directory: {:?}", rc_file);
        if let Ok(f) = std::fs::File::open(rc_file) {
            let mut lines: Vec<String> = std::io::BufReader::new(f)
                .lines()
                .map(|e| e.unwrap())
                .collect();
            lines.insert(0, "DUMMY".to_string()); // normally the first element holds the program name
            Opt::from_iter(lines.iter())
        } else {
            Opt::default()
        }
    };
    let opt_from_args = Opt::from_args();

    if opt_from_args.debug || opt_from_file.debug {
        println!("file={:?} cmd={:?}", opt_from_file, opt_from_args);
    }
    let mut files = opt_from_args.files;
    files.append(&mut opt_from_file.files);
    let opt = Opt {
        holidays: first_available(opt_from_args.holidays, opt_from_file.holidays),
        debug: opt_from_args.debug || opt_from_file.debug,
        verbose: opt_from_args.verbose || opt_from_file.verbose,
        be_lenient: opt_from_args.be_lenient || opt_from_file.be_lenient,
        duration_of_day: first_available(
            opt_from_args.duration_of_day,
            opt_from_file.duration_of_day,
        ),
        timezone: first_available(opt_from_args.timezone, opt_from_file.timezone),
        log_to_jira: opt_from_args.log_to_jira, // here we actually ignore the options from the file
        jira_base_url: first_available(opt_from_args.jira_base_url, opt_from_file.jira_base_url),
        jira_username: first_available(opt_from_args.jira_username, opt_from_file.jira_username),
        jira_password: first_available(opt_from_args.jira_password, opt_from_file.jira_password),
        files,
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
        if let Some(ref e) = work_day_by_date.insert(day_raw.date, (*day_raw).clone()) {
            println!("Duplicate day: {:?}", e);
            has_error = true;
        }
    }
    if has_error {
        println!("Abort because of errors");
        return;
    };
    let first_date = &work_days_raw[0].as_ref().unwrap().date;
    let (min_day, max_day) =
        work_days_raw
            .iter()
            .fold((*first_date, *first_date), |(min, max), day| {
                let date = &day.as_ref().unwrap().date;
                (std::cmp::min(min, *date), std::cmp::max(max, *date))
            });
    if opt.debug {
        println!("min={} max={}", min_day.format("%F"), max_day.format("%F"));
    }
    let duration_of_day = if let Some(d) = opt.duration_of_day {
        d
    } else {
        chrono::Duration::hours(8)
    };
    let required_time = match opt.holidays {
        Some(fp) => {
            let required_time = log_work::required_time::parse_required_time_file(&fp)
                .expect("Error parsing required time file");
            log_work::required_time::consolidate_required_time(
                &required_time,
                &min_day,
                &max_day,
                &duration_of_day,
            )
            .expect("Failed to consolidate required times")
        }
        None => {
            let empty = Vec::new();
            log_work::required_time::consolidate_required_time(
                &empty,
                &min_day,
                &max_day,
                &duration_of_day,
            )
            .expect("Failed to consolidate required times")
        }
    };
    if opt.debug {
        println!("Required-times: {:?}", required_time);
    }

    let days = log_work::work_day::Days::join_work_and_requirement(
        &work_day_by_date,
        &required_time,
        &duration_of_day,
    );
    if opt.debug {
        for ref day in &days.days {
            println!("Required-times for {:?}: {:?}", day.required_time.date, day);
        }
    }

    let mut summary = log_work::work_day::Summary::new();
    let mut sum_required = chrono::Duration::hours(0);
    for day in &days.days {
        println!(
            "{}",
            log_work::work_day::DaySummary {
                day,
                verbose: opt.verbose
            }
        );
        log_work::work_day::WorkDay::merge_summaries_right_into_left(
            &mut summary,
            &day.work_day.compute_summary(),
        );
        sum_required += day.required_time.required_time;
    }
    if days.days.len() > 1 {
        println!("= Summary for all days:");
        let mut sum = chrono::Duration::hours(0);
        for (key, duration) in summary.iter() {
            println!(
                "{:20}:{:>20}",
                key,
                log_work::util::WorkDuration {
                    duration_of_day,
                    duration: *duration
                }
            );
            if key != "Pause" {
                sum += *duration;
            }
        }
        println!(
            "{:20}: {:>20}",
            " == Required ==",
            log_work::util::WorkDuration {
                duration: sum_required,
                duration_of_day
            }
        );
        println!(
            "{:20}: {:>20}",
            " == Total ==",
            log_work::util::WorkDuration {
                duration_of_day,
                duration: sum
            }
        );
    }
    if opt.log_to_jira {
        if opt.be_lenient {
            println!("ERROR: Updating JIRA-logging is forbidden in lenient mode!");
        } else {
            let timezone = if let Some(tz) = opt.timezone {
                log_work::jira::TimeZone::Tz(tz)
            } else {
                log_work::jira::TimeZone::Local(chrono::Local)
            };
            let jira_config = log_work::jira::JiraConfig {
                base_url: opt.jira_base_url.expect("Missing JIRA base URL"),
                username: opt.jira_username.expect("Missing JIRA username"),
                password: opt.jira_password.clone(),
                timezone,
            };

            let result = log_work::jira::update_logging_for_days(
                &days.days.iter().map(|day| &day.work_day).collect(),
                &jira_config,
            );
            match result {
                Ok(()) => {
                    println!("Successfully updated JIRA time logging");
                }
                Err(e) => {
                    println!(
                        "Sending the data to JIRA yielded the following result: {:?}",
                        e
                    );
                }
            }
        }
    }
}
