//! User aggregate root — pure domain. No DB, no axum.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::errors::{DomainError, ValidationError};


// Value objects

/// Two-letter region code per ISO 3166-1 alpha-2 (RU, US, ...). Stored
/// uppercase. We rely on this for jurisdiction-specific behaviour
/// (renewal rules, ammo limits, legal documents).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region(String);

impl Region {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value: String = value.into();
        let value = value.trim().to_uppercase();
        if value.len() != 2 || !value.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(ValidationError::Region(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    RU,
    EN,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::RU => "ru",
            Language::EN => "en",
        }
    }
}

impl std::str::FromStr for Language {
    type Err = ValidationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "ru" | "russian" => Ok(Language::RU),
            "en" | "english" => Ok(Language::EN),
            other => Err(ValidationError::Language(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Pending,
    Verified,
    Blocked,
}

impl UserStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserStatus::Pending => "pending",
            UserStatus::Verified => "verified",
            UserStatus::Blocked => "blocked",
        }
    }
}

impl std::str::FromStr for UserStatus {
    type Err = ValidationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "verified" => Ok(Self::Verified),
            "blocked" => Ok(Self::Blocked),
            other => Err(ValidationError::Custom(format!(
                "unknown user_status `{other}`"
            ))),
        }
    }
}

//    Entity    

/// Domain entity. Construct only through [`User::register`] (for new
/// users) or [`User::rehydrate`] (when loading from the repository).
/// The fields are private so invariants can't be broken by callers.
#[derive(Debug, Clone)]
pub struct User {
    id: Uuid,
    login: String,
    firstname: String,
    surname: String,
    email: String,
    date_of_birth: NaiveDate,
    region: Region,
    language: Language,
    status: UserStatus,
    is_admin: bool,
    last_visit_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl User {
    pub const DEFAULT_MINIMUM_AGE: i32 = 18;

    /// Smart constructor for *new* users — generates id + timestamps,
    /// enforces minimum age, sets status = Pending. The only path
    /// through which `User` can come into existence on the registration
    /// flow.
    pub fn register(
        login: String,
        firstname: String,
        surname: String,
        email: String,
        date_of_birth: NaiveDate,
        region: Region,
        language: Language,
    ) -> Result<Self, DomainError> {
        let now = Utc::now();
        let user = Self {
            id: Uuid::new_v4(),
            login,
            firstname,
            surname,
            email,
            date_of_birth,
            region,
            language,
            status: UserStatus::Pending,
            is_admin: false,
            last_visit_at: None,
            created_at: now,
            updated_at: now,
        };

        if !user.is_of_legal_age() {
            return Err(DomainError::Underage {
                required: Self::DEFAULT_MINIMUM_AGE,
                actual: user.age(),
            });
        }
        Ok(user)
    }

    /// Rehydrate from persistence. Bypasses validation; the DB row is
    /// trusted because it was validated on insert. Used by repositories.
    #[allow(clippy::too_many_arguments)]
    pub fn rehydrate(
        id: Uuid,
        login: String,
        firstname: String,
        surname: String,
        email: String,
        date_of_birth: NaiveDate,
        region: Region,
        language: Language,
        status: UserStatus,
        is_admin: bool,
        last_visit_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            login,
            firstname,
            surname,
            email,
            date_of_birth,
            region,
            language,
            status,
            is_admin,
            last_visit_at,
            created_at,
            updated_at,
        }
    }

    // -------------------- field accessors --------------------- //

    pub fn id(&self) -> Uuid { self.id }
    pub fn login(&self) -> &str { &self.login }
    pub fn firstname(&self) -> &str { &self.firstname }
    pub fn surname(&self) -> &str { &self.surname }
    pub fn email(&self) -> &str { &self.email }
    pub fn date_of_birth(&self) -> NaiveDate { self.date_of_birth }
    pub fn region(&self) -> &Region { &self.region }
    pub fn language(&self) -> Language { self.language }
    pub fn status(&self) -> UserStatus { self.status }
    pub fn is_admin(&self) -> bool { self.is_admin }
    pub fn last_visit_at(&self) -> Option<DateTime<Utc>> { self.last_visit_at }
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
    pub fn updated_at(&self) -> DateTime<Utc> { self.updated_at }

    // -------------------- domain operations -------------------- //

    /// Returns the user's age in **completed years**.
    ///
    /// (The previous implementation had an off-by-one bug: it
    /// *subtracted* a year when the current date was past the birthday
    /// instead of *before* it.)
    pub fn age(&self) -> i32 {
        let today = Utc::now().date_naive();
        let mut age = today.year() - self.date_of_birth.year();

        // If today is BEFORE this year's birthday, subtract one.
        if (today.month(), today.day())
            < (self.date_of_birth.month(), self.date_of_birth.day())
        {
            age -= 1;
        }
        age
    }

    pub fn is_of_legal_age(&self) -> bool {
        self.age() >= Self::DEFAULT_MINIMUM_AGE
    }

    pub fn is_verified(&self) -> bool {
        matches!(self.status, UserStatus::Verified)
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self.status, UserStatus::Blocked)
    }

    /// State transition: Pending - Verified.
    pub fn mark_verified(&mut self) -> Result<(), DomainError> {
        match self.status {
            UserStatus::Pending => {
                self.status = UserStatus::Verified;
                self.updated_at = Utc::now();
                Ok(())
            }
            UserStatus::Verified => Err(DomainError::AlreadyVerified),
            UserStatus::Blocked => Err(DomainError::Blocked),
        }
    }

    /// Update the "last visit" timestamp — called on every successful
    /// authenticated request (or on login, depending on policy).
    pub fn touch_visit(&mut self) {
        let now = Utc::now();
        self.last_visit_at = Some(now);
        self.updated_at = now;
    }

    pub fn block(&mut self) {
        self.status = UserStatus::Blocked;
        self.updated_at = Utc::now();
    }
}


//     Tests    


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn new_adult() -> Result<User, DomainError> {
        User::register(
            "jdoe".into(),
            "John".into(),
            "Doe".into(),
            "j@d.com".into(),
            NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
            Region::new("RU").unwrap(),
            Language::RU,
        )
    }

    #[test]
    fn register_adult_ok() {
        assert!(new_adult().is_ok());
    }

    #[test]
    fn register_minor_rejected() {
        let today = Utc::now().date_naive();
        let dob = NaiveDate::from_ymd_opt(today.year() - 10, 1, 1).unwrap();
        let err = User::register(
            "kid".into(), "K".into(), "K".into(), "k@k.com".into(),
            dob, Region::new("RU").unwrap(), Language::RU,
        ).unwrap_err();
        assert!(matches!(err, DomainError::Underage { .. }));
    }

    #[test]
    fn mark_verified_transitions() {
        let mut u = new_adult().unwrap();
        assert!(!u.is_verified());
        u.mark_verified().unwrap();
        assert!(u.is_verified());
        assert!(matches!(
            u.mark_verified(),
            Err(DomainError::AlreadyVerified)
        ));
    }

    #[test]
    fn region_validation() {
        assert!(Region::new("RU").is_ok());
        assert!(Region::new("ru").is_ok());
        assert!(Region::new("RUS").is_err());
        assert!(Region::new("R1").is_err());
    }
}
