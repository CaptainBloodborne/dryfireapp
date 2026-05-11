use anyhow::Ok;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub database_url: String,
    pub controller: String,
}

pub fn init_config() -> anyhow::Result<Config> {
    let config = envy::from_env::<Config>()?;

    Ok(config)

}