//! This module contain application configuration related functionality.
//! 
//! All the application configurations should be set in corresponding
//! TOML file in `config` directory.
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::fmt;
use crate::string_util::StrUtil;

const DEFAULT_CONFIG_FILE_PREFIX: &str = "./config";
const DEFAULT_CONFIG_FILE_NAME: &str = "default.toml";

fn default_config_file_path(base_path: &str) -> String {
    format!("{}/{}", base_path.trim_right_slash(), DEFAULT_CONFIG_FILE_NAME)
}

#[derive(Debug, Deserialize, Clone)]
pub struct HttpServer {
    pub enabled: bool,
    pub port: u16
}

#[derive(Debug, Deserialize, Clone)]
pub struct DasCfg {
    pub enabled: bool,
    pub grpc_address: String,
    pub fetch_batch_size: u32,
    pub number_of_workers: usize,
}

#[derive(Deserialize, Clone)]
pub struct ObjStorage {
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub bucket_for_media: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AssetProcessorCfg {
    pub resize_to: u32,
    pub file_max_size_bytes: u64,
}

impl fmt::Debug for ObjStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjStorage")
            .field("region", &self.region)
            .field("endpoint", &self.endpoint)
            .field("access_key_id", &self.access_key_id.as_ref().map(|s|mask_creds(s)))
            .field("secret_access_key", &self.secret_access_key.as_ref().map(|s|mask_creds(s)))
            .field("session_token", &self.session_token.as_ref().map(|s|mask_creds(s)))
            .field("bucket_for_media", &self.bucket_for_media)
            .finish()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Metrics {
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub http_server: HttpServer,
    pub obj_storage: ObjStorage,
    pub asset_processor: AssetProcessorCfg,
    pub das: DasCfg,
    pub env: String,
    pub metrics: Metrics,
}

impl Settings {
    pub fn for_env(env_name: &str) -> Result<Self, ConfigError> {
        Settings::load(Some(env_name), None)
    }

    /// This method should be used for production.
    /// It loads application configuration based on the environment variables.
    pub fn default() -> Result<Self, ConfigError> {
        Settings::load(None, None)
    }

    fn load(env_name: Option<&str>, config_path: Option<&str>) -> Result<Self, ConfigError> {

        let configs_path = config_path.map(|s|s.to_string()).unwrap_or(
            std::env::var("RUN_CONFIG_DIR").unwrap_or_else(|_| DEFAULT_CONFIG_FILE_PREFIX.to_string())
        );

        let env = env_name.map(|s|s.to_string()).unwrap_or(
            std::env::var("RUN_ENV").unwrap_or_else(|_| "local".into())
        );
        println!("Using profile: {}", &env);

        let raw_config = Config::builder()
            // Start off by merging in the "default" configuration file
            .add_source(File::with_name(&default_config_file_path(&configs_path)))
            // Add in the current environment file, Default to 'development' env
            // Note that this file is _optional_
            .add_source(
                File::with_name(&format!("{}/{}", configs_path, env)).required(false),
            )
            // Add in settings from the environment (with a prefix of APP)
            // Eg.. `APP_SERVER__PORT=8081 ./target/app` would set the `port` key
            .add_source(Environment::with_prefix("app").separator("__"))
            .set_override("env", env)?
            .build()?;

        raw_config.try_deserialize()
    }
}

fn mask_creds(s: &str) -> String {
    let mut result = s.to_owned();
    result.replace_range( 2 .. s.len(), "*".repeat(s.len()-2).as_str());
    result
}