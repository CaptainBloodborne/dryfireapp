use async_trait::async_trait;

use crate::domain::{entities::user::User, services::identity::Credentials};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn save_user(user: User, password: Credentials) -> anyhow::Result<()>;
    async fn is_email_exist(email: &str) -> bool;
    async fn is_login_exist(login: &str) -> bool;

}