use std::sync::OnceLock;

use lib_utils::envs::get_env;

pub fn config() -> &'static Config {
    static INSTANCE: OnceLock<Config> = OnceLock::new();

    INSTANCE.get_or_init(|| {
        Config::load_from_env()
            .unwrap_or_else(|ex| panic!("FATAL - WHILE LOADING CONFIG - Cause:{:?}", ex))
    })
}

#[allow(non_snake_case)]
pub struct Config {
    pub DB_URL: String,
    pub TEST_DB_URL: String,
}

impl Config {
    pub fn load_from_env() -> lib_utils::Result<Config> {
        Ok(Config {
            DB_URL: get_env("SERVICE_DB_URL")?,
            TEST_DB_URL: get_env("SERVICE_TEST_DB_URL")?,
        })
    }
}
