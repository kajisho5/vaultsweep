//! 自動診断の予定（Windows のタスク スケジューラ）

use crate::store::Settings;
use chrono::{Datelike, Duration, Local, NaiveDateTime, NaiveTime, Weekday};

pub const TASK_NAME: &str = "vaultsweep\\scan";

/// schtasks に渡す引数（作成）
pub fn create_args(exe: &str, s: &Settings) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "/Create".into(),
        "/TN".into(),
        TASK_NAME.into(),
        "/TR".into(),
        format!("\"{exe}\" --scan"),
        "/ST".into(),
        s.schedule_time.clone(),
        "/F".into(),
    ];
    match s.schedule.as_str() {
        "daily" => a.extend(["/SC".into(), "DAILY".into()]),
        "monthly" => a.extend(["/SC".into(), "MONTHLY".into(), "/D".into(), "1".into()]),
        _ => a.extend(["/SC".into(), "WEEKLY".into(), "/D".into(), "MON".into()]),
    }
    a
}

pub fn delete_args() -> Vec<String> {
    vec![
        "/Delete".into(),
        "/TN".into(),
        TASK_NAME.into(),
        "/F".into(),
    ]
}

/// 次に診断する日時
pub fn next_run(s: &Settings, now: NaiveDateTime) -> Option<NaiveDateTime> {
    if !s.auto_scan {
        return None;
    }
    let t = NaiveTime::parse_from_str(&s.schedule_time, "%H:%M").ok()?;
    let today = now.date();
    for add in 0..62 {
        let d = today + Duration::days(add);
        let ok = match s.schedule.as_str() {
            "daily" => true,
            "monthly" => d.day() == 1,
            _ => d.weekday() == Weekday::Mon,
        };
        let at = d.and_time(t);
        if ok && at > now {
            return Some(at);
        }
    }
    None
}

pub fn next_run_now(s: &Settings) -> Option<String> {
    next_run(s, Local::now().naive_local()).map(|d| d.format("%Y-%m-%dT%H:%M").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn next() {
        let s = Settings {
            auto_scan: true,
            ..Default::default()
        };
        // 2026-10-25 は日曜
        let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 25)
            .unwrap()
            .and_hms_opt(17, 0, 0)
            .unwrap();
        assert_eq!(
            next_run(&s, now).unwrap().to_string(),
            "2026-10-26 09:00:00"
        );
        let m = Settings {
            auto_scan: true,
            schedule: "monthly".into(),
            ..Default::default()
        };
        assert_eq!(
            next_run(&m, now).unwrap().to_string(),
            "2026-11-01 09:00:00"
        );
        let a = create_args("C:\\x\\vaultsweep.exe", &s);
        assert!(a.contains(&"WEEKLY".to_string()));
    }
}
