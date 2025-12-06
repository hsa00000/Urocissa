use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use rand::Rng;
use regex::Regex;

use std::{path::Path, sync::LazyLock};

use crate::{
    public::structure::abstract_data::AbstractData,
    public::structure::database::file_modify::FileModify, workflow::tasks::actor::index::IndexTask,
};

static FILE_NAME_TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(\d{4})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})\b").unwrap()
});

pub fn compute_timestamp_ms_by_exif(index_task: &mut IndexTask) -> () {
    let now_time = Local::now().naive_local();

    // 嘗試從 EXIF 解析 DateTimeOriginal
    if let Some(value) = index_task.exif_vec.get("DateTimeOriginal")
        && let Ok(naive_dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        && let Some(local_dt) = Local.from_local_datetime(&naive_dt).single()
        && local_dt.naive_local() <= now_time
    {
        let timestamp = local_dt.timestamp_millis();
        // 更新 AbstractData 中的 created_time
        match &mut index_task.data {
            AbstractData::Image(img) => {
                img.object.created_time = timestamp;
            }
            AbstractData::Video(vid) => {
                vid.object.created_time = timestamp;
            }
            _ => {}
        }
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
                let path = Path::new(&file_modify.file);

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
