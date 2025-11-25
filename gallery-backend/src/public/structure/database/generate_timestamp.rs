use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use log::info;
use rand::Rng;
use regex::Regex;

use std::{collections::BTreeMap, path::PathBuf, sync::LazyLock};

use crate::public::structure::database::file_modify::FileModify;

static FILE_NAME_TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(\d{4})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})\b").unwrap()
});

pub fn compute_timestamp_ms_by_exif(exif_vec: &BTreeMap<String, String>) -> Option<i64> {
    let now_time = Local::now().naive_local();

    if let Some(value) = exif_vec.get("DateTimeOriginal")
        && let Ok(naive_dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        && let Some(local_dt) = Local.from_local_datetime(&naive_dt).single()
        && local_dt.naive_local() <= now_time
    {
        info!(
            "local_dt.timestamp_millis() is {:?}",
            local_dt.timestamp_millis()
        );
        Some(local_dt.timestamp_millis())
    } else {
        None
    }
}

pub fn compute_timestamp_ms_by_file_modify(
    file_modify: &FileModify,
    priority_list: &[&str],
) -> i64 {
    let now_time = chrono::Local::now().naive_local();

    for &field in priority_list {
        match field {
            "filename" => {
                let path = PathBuf::from(&file_modify.file);

                if let Some(file_name) = path.file_name()
                    && let Some(caps) = FILE_NAME_TIME_REGEX.captures(file_name.to_str().unwrap())
                    && let (Ok(year), Ok(month), Ok(day), Ok(hour), Ok(minute), Ok(second)) = (
                        caps[1].parse::<i32>(),
                        caps[2].parse::<u32>(),
                        caps[3].parse::<u32>(),
                        caps[4].parse::<u32>(),
                        caps[5].parse::<u32>(),
                        caps[6].parse::<u32>(),
                    )
                    && let Some(date) = NaiveDate::from_ymd_opt(year, month, day)
                    && let Some(time) = NaiveTime::from_hms_opt(hour, minute, second)
                {
                    let datetime = NaiveDateTime::new(date, time);

                    if datetime <= now_time {
                        return chrono::Local
                            .from_local_datetime(&datetime)
                            .unwrap()
                            .timestamp_millis();
                    }
                }
            }
            "scan_time" => {
                return file_modify.scan_time as i64;
            }
            "modified" => {
                return file_modify.modified as i64;
            }
            "random" => {
                let mut rng = rand::rng();
                let random_number: i64 = rng.random();
                return random_number;
            }
            _ => panic!("Unknown field type: {}", field),
        }
    }

    0
}
