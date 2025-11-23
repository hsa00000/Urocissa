use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use rand::Rng;
use regex::Regex;

use std::{path::PathBuf, sync::LazyLock};

use super::definition::Database;

static FILE_NAME_TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(\d{4})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})\b").unwrap()
});

impl Database {
    pub fn compute_timestamp(&self, priority_list: &[&str]) -> u128 {
        let now_time = chrono::Local::now().naive_local();
        for &field in priority_list {
            match field {
                "DateTimeOriginal" => {
                    if let Some(value) = self.exif_vec.get("DateTimeOriginal")
                        && let Ok(naive_dt) =
                            NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                        && let Some(local_dt) =
                            chrono::Local.from_local_datetime(&naive_dt).single()
                        && local_dt.naive_local() <= now_time
                    {
                        return local_dt.timestamp_millis() as u128;
                    }
                }
                "filename" => {
                    let mut max_time: Option<NaiveDateTime> = None;

                    for alias in &self.alias {
                        let path = PathBuf::from(&alias.file);

                        if let Some(file_name) = path.file_name()
                            && let Some(caps) =
                                FILE_NAME_TIME_REGEX.captures(file_name.to_str().unwrap())
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
                                max_time = Some(max_time.map_or(datetime, |t| t.max(datetime)));
                            }
                        }
                    }

                    if let Some(datetime) = max_time {
                        return chrono::Local
                            .from_local_datetime(&datetime)
                            .unwrap()
                            .timestamp_millis() as u128;
                    }
                }
                "scan_time" => {
                    let latest_scan_time = self.alias.iter().map(|alias| alias.scan_time).max();
                    if let Some(latest_time) = latest_scan_time {
                        return latest_time as u128;
                    }
                }
                "modified" => {
                    if let Some(max_scan_alias) =
                        self.alias.iter().max_by_key(|alias| alias.scan_time)
                    {
                        return max_scan_alias.modified;
                    }
                }
                "random" => {
                    let mut rng = rand::rng();
                    let random_number: u128 = rng.random();
                    return random_number;
                }
                _ => panic!("Unknown field type: {}", field),
            }
        }
        0
    }
}
