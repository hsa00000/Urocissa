use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use rand::Rng;
use regex::Regex;

use std::{path::PathBuf, sync::LazyLock};

use super::definition::Database;
use crate::public::db::tree::TREE;
use crate::public::structure::database_struct::file_modify::FileModify;

static FILE_NAME_TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(\d{4})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})\b").unwrap()
});

impl Database {
    pub fn compute_timestamp_ms(&self, priority_list: &[&str]) -> i64 {
        let conn = match TREE.get_connection() {
            Ok(c) => c,
            Err(_) => return 0,
        };
        let mut stmt = conn.prepare("SELECT file, modified, scan_time FROM database_alias WHERE hash = ?").expect("Failed to prepare");
        let alias_iter = stmt.query_map([self.hash.as_str()], |row| {
            Ok(FileModify {
                file: row.get(0)?,
                modified: row.get::<_, i64>(1)? as u128,
                scan_time: row.get::<_, i64>(2)? as u128,
            })
        }).expect("Failed to query");
        let alias: Vec<FileModify> = alias_iter.filter_map(|r| r.ok()).collect();

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
                        return local_dt.timestamp_millis();
                    }
                }
                "filename" => {
                    let mut max_time: Option<NaiveDateTime> = None;

                    for file_modify in &alias {
                        let path = PathBuf::from(&file_modify.file);

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
                            .timestamp_millis();
                    }
                }
                "scan_time" => {
                    let latest_scan_time = alias.iter().map(|file_modify| file_modify.scan_time).max();
                    if let Some(latest_time) = latest_scan_time {
                        return latest_time as i64;
                    }
                }
                "modified" => {
                    if let Some(max_scan_alias) =
                        alias.iter().max_by_key(|file_modify| file_modify.scan_time)
                    {
                        return max_scan_alias.modified as i64;
                    }
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
}
