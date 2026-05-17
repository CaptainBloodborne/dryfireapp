use secrecy::SecretString;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NewUserPayload {
    pub login: String,
    pub email: String,
    pub password: SecretString,
}

