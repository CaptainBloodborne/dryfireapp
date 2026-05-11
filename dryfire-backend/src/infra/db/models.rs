use chrono::NaiveDate;
use sqlx::FromRow;
use uuid::Uuid;

use crate::domain::entities::user::User;

#[derive(Debug, FromRow)]
struct UserDB {
    pub id: Uuid,
    pub login: String,
    pub firstname: String,
    pub surname: String,
    pub email: String,
    pub date_of_birth: NaiveDate,
    pub language: String,
}

impl From<UserDB> for User {
    fn from(value: UserDB) -> Self {
        let language = value.language.as_str();
        User::new(
            value.id,
            value.login,
            value.firstname,
            value.surname,
            value.email,
            value.date_of_birth,
            language.into(),
        )
    }
}
