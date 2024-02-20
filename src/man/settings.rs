use std::default::Default;
use std::net::SocketAddr;

use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::db;
use crate::result::{Error, Result};
use crate::web::server::{self, to_res};

const TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("settings");
pub(crate) const SETTINGS_KEY: &str = "settings";

#[derive(Deserialize, Serialize)]
pub(crate) struct Settings {
    pub(crate) ip: String,
    pub(crate) port: u16,
    #[serde(rename = "maxSessionDurationMin")]
    pub(crate) max_session_duration_min: u16,
    #[serde(rename = "smtpHost")]
    pub(crate) smtp_host: String,
    #[serde(rename = "smtpUsername")]
    pub(crate) smtp_username: String,
    #[serde(rename = "smtpPassword")]
    pub(crate) smtp_password: String,
    #[serde(rename = "emailVerificationRegex")]
    pub(crate) email_verification_regex: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            ip: String::from("127.0.0.1"),
            port: 12715,
            max_session_duration_min: 30,
            smtp_host: String::new(),
            smtp_username: String::new(),
            smtp_password: String::new(),
            email_verification_regex: String::new(),
        }
    }
}

pub(crate) fn init_table() -> Result<()> {
    db::init_table(TABLE)
}

pub(crate) fn exists() -> Result<bool> {
    let cnt = db::count(TABLE)?;
    Ok(cnt > 0)
}

pub(crate) fn init() -> Result<Settings> {
    let settings = Settings::default();
    db::write(TABLE, SETTINGS_KEY, &settings)?;
    let format = time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]]")
        .expect("Invalid format description");
    let t = time::OffsetDateTime::now_utc();
    let t_str = t.format(&format).map_err(|e| Error::TimeFormatError(e))?;
    db::write(TABLE, "db_init_time", &t_str)?;
    db::write(TABLE, "version", &String::from(server::VERSION))?;
    Ok(settings)
}

pub(crate) fn get_settings() -> Result<Option<Settings>> {
    db::query(TABLE, SETTINGS_KEY)
}

pub(crate) async fn get() -> impl IntoResponse {
    to_res::<Option<Settings>>(get_settings())
}

pub(crate) async fn save(Json(data): Json<Settings>) -> impl IntoResponse {
    to_res(save_settings(&data))
}

pub(crate) fn save_settings(data: &Settings) -> Result<()> {
    let addr = format!("{}:{}", data.ip, data.port);
    let _: SocketAddr = addr.parse().map_err(|_| {
        log::error!("Saving invalid listen IP: {}", &addr);
        Error::ErrorWithMessage(String::from("lang.settings.invalidIp"))
    })?;
    db::write(TABLE, SETTINGS_KEY, &data)
}
