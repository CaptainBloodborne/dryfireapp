use chrono::{DateTime, Duration, TimeZone, Utc};

pub fn utc_now() -> DateTime<Utc> {
    Utc::now()
}

pub fn format_time<T: TimeZone>(time: DateTime<T>) -> String {
    time.to_rfc3339()
}

pub fn now_utc_plus_sec_str(sec: i64) -> String {
    let t = utc_now() + Duration::seconds(sec);
    format_time(t)
}

pub fn now_utc_plus_sec(sec: i64) -> DateTime<Utc> {
    utc_now() + Duration::seconds(sec)
}

pub fn parse_utc(time: &str) -> anyhow::Result<DateTime<Utc>> {
    let t = DateTime::parse_from_rfc3339(time)?;
    Ok(t.to_utc())
}
