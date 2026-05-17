#![allow(unused)]
use anyhow::anyhow;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use regex::Regex;
use tower_cookies::cookie::time::error;
use uuid::Uuid;

use crate::utils::time::utc_now;


pub enum Language {
    RU,
    EN,
}

pub enum UserStatus {
    Pending,
    Verified,
    Blocked,

}

impl From<&str> for Language {
    fn from(value: &str) -> Self {
        match value {
            "russian" => Language::RU,
            "english" => Language::EN,
            _ => Language::EN,
        }
    }
}


pub struct User {
    id: Uuid,
    login: String,
    firstname: String,
    surname: String,
    email: String,
    date_of_birth: NaiveDate,
    status: UserStatus,
    language: Language,
}

impl User {
    const DEFAULT_MINIMUM_AGE: i32 = 18;

    pub fn new(
        id: Uuid,
        login: String,
        firstname: String,
        surname: String,
        email: String,
        date_of_birth: NaiveDate,
        language: Language,
    ) -> Self {
        Self {
            id,
            login,
            firstname,
            surname,
            email,
            date_of_birth,
            language,
            status: UserStatus::Pending,
        }
    }

    fn get_age(&self) -> i32 {
        let current_date = Utc::now().date_naive();
        let mut age = current_date.year() - self.date_of_birth.year();

        if (current_date.month(), current_date.day()) > (self.date_of_birth.month(), self.date_of_birth.day()) {
            age -= 1;
        }

        age
    }

    fn check_if_age_is_legal(&self) -> bool {
        let current_date = Utc::now().date_naive();
        let mut age = current_date.year() - self.date_of_birth.year();

        if (current_date.month(), current_date.day()) > (self.date_of_birth.month(), self.date_of_birth.day()) {
            age -= 1;
        }

        return age >= Self::DEFAULT_MINIMUM_AGE;

    }

    fn check_if_user_verified(&self) -> bool {
        matches!(self.status, UserStatus::Pending)
    }
}
