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


pub enum TimeError {
    ParsingError
}