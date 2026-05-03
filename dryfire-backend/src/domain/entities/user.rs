use chrono::{DateTime, Utc};

use crate::utils::time::utc_now;

pub enum Language {
    RU,
    EN,
}

pub struct User {
    login: String,
    first_name: String,
    surname: String,
    date_of_birth: DateTime<Utc>,
    language: Language,
    last_visit: DateTime<Utc>,
}

impl User {
    pub fn new(
        login: String,
        first_name: String,
        surname: String,
        email: String,
        date_of_birth: DateTime<Utc>,
        language: Language,
    ) -> Self {
        Self {
            login,
            first_name,
            surname,
            date_of_birth,
            language,
            last_visit: utc_now(),
        }
    }

    fn check_if_age_is_legal(&self) -> bool {
        todo!()
    }
}
