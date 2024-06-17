use std::collections::HashMap;
use std::default::Default;
use std::net::SocketAddr;

use axum::body::Bytes;
use axum::extract::Query;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::ai::huggingface::HuggingFaceModel;
use crate::ai::{completion, embedding, huggingface};
use crate::db;
use crate::result::{Error, Result};
use crate::robot::dto::RobotQuery;
use crate::web::server::{self, to_res};

pub(crate) const TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("settings");
pub(crate) const SETTINGS_KEY: &str = "global-settings";

#[derive(Deserialize, Serialize)]
pub(crate) struct HfModelDownload {
    #[serde(rename = "connectTimeoutMillis")]
    pub(crate) connect_timeout_millis: u16,
    #[serde(rename = "readTimeoutMillis")]
    pub(crate) read_timeout_millis: u16,
    #[serde(rename = "accessToken")]
    pub(crate) access_token: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct GlobalSettings {
    pub(crate) ip: String,
    pub(crate) port: u16,
    #[serde(rename = "selectRandomPortWhenConflict")]
    pub(crate) select_random_port_when_conflict: bool,
    #[serde(rename = "HfModelDownload")]
    pub(crate) hf_model_download: HfModelDownload,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Settings {
    #[serde(rename = "maxSessionIdleSec")]
    pub(crate) max_session_idle_sec: u32,
    #[serde(rename = "textGenerationProvider")]
    pub(crate) text_generation_provider: TextGenerationProvider,
    #[serde(rename = "sentenceEmbeddingProvider")]
    pub(crate) sentence_embedding_provider: SentenceEmbeddingProvider,
    #[serde(rename = "smtpHost")]
    pub(crate) smtp_host: String,
    #[serde(rename = "smtpUsername")]
    pub(crate) smtp_username: String,
    #[serde(rename = "smtpPassword")]
    pub(crate) smtp_password: String,
    #[serde(rename = "smtpTimeoutSec")]
    pub(crate) smtp_timeout_sec: u16,
    #[serde(rename = "emailVerificationRegex")]
    pub(crate) email_verification_regex: String,
}

// #[test]
// fn deser() {
//     let s = Settings {
//         ip: String::new(),
//         port: 12715,
//         max_session_duration_min: 60,
//         embedding_provider: EmbeddingProvider {
//             provider: crate::intent::embedding::EmbeddingProvider::HuggingFace(
//                 crate::intent::embedding::HuggingFaceModel::AllMiniLML6V2,
//             ),
//             api_url: String::new(),
//             api_key: String::new(),
//             model: String::new(),
//             connect_timeout_millis: 1000,
//             read_timeout_millis: 5000,
//         },
//         smtp_host: String::new(),
//         smtp_username: String::new(),
//         smtp_password: String::new(),
//         smtp_timeout_sec: 30,
//         email_verification_regex: String::new(),
//         select_random_port_when_conflict: false,
//     };
//     let j = serde_json::to_string(&s);
//     assert!(j.is_ok());
//     println!("{}", j.unwrap());
//     let j = "{\"ip\":\"127.0.0.1\",\"port\":12715,\"selectRandomPortWhenConflict\":false,\"maxSessionDurationMin\":30,\"smtpHost\":\"\",\"smtpUsername\":\"\",\"smtpPassword\":\"\",\"smtpTimeoutSec\":60,\"emailVerificationRegex\":\"[-\\w\\.\\+]{1,100}@[A-Za-z0-9]{1,30}[A-Za-z\\.]{2,30}\",\"embeddingProvider\":{\"provider\":\"HuggingFace\",\"apiUrl\":\"Model will be downloaded locally at ./data/models\",\"apiKey\":\"\",\"model\":\"AllMiniLML6V2\",\"apiUrlDisabled\":true,\"showApiKeyInput\":false}}";
//     let r = serde_json::from_str(j);
//     assert!(r.is_ok());
//     let v: serde_json::Value = r.unwrap();
//     assert_eq!(v["embeddingProvider"]["provider"], "HuggingFace");
// }

#[derive(Deserialize, Serialize)]
pub(crate) struct TextGenerationProvider {
    pub(crate) provider: completion::TextGenerationProvider,
    #[serde(rename = "apiUrl")]
    pub(crate) api_url: String,
    #[serde(rename = "apiKey")]
    pub(crate) api_key: String,
    pub(crate) model: String,
    #[serde(rename = "connectTimeoutMillis")]
    pub(crate) connect_timeout_millis: u16,
    #[serde(rename = "readTimeoutMillis")]
    pub(crate) read_timeout_millis: u16,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct SentenceEmbeddingProvider {
    pub(crate) provider: embedding::SentenceEmbeddingProvider,
    #[serde(rename = "apiUrl")]
    pub(crate) api_url: String,
    #[serde(rename = "apiKey")]
    pub(crate) api_key: String,
    pub(crate) model: String,
    #[serde(rename = "connectTimeoutMillis")]
    pub(crate) connect_timeout_millis: u16,
    #[serde(rename = "readTimeoutMillis")]
    pub(crate) read_timeout_millis: u16,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        GlobalSettings {
            ip: String::from("127.0.0.1"),
            port: 12715,
            select_random_port_when_conflict: false,
            hf_model_download: HfModelDownload {
                connect_timeout_millis: 1000,
                read_timeout_millis: 10000,
                access_token: String::new(),
            },
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            max_session_idle_sec: 1800,
            text_generation_provider: TextGenerationProvider {
                provider: completion::TextGenerationProvider::HuggingFace(
                    huggingface::HuggingFaceModel::Phi3Mini4kInstruct,
                ),
                api_url: String::new(),
                api_key: String::new(),
                model: String::new(),
                connect_timeout_millis: 1500,
                read_timeout_millis: 3000,
            },
            sentence_embedding_provider: SentenceEmbeddingProvider {
                provider: embedding::SentenceEmbeddingProvider::HuggingFace(
                    huggingface::HuggingFaceModel::AllMiniLML6V2,
                ),
                api_url: String::new(),
                api_key: String::new(),
                model: String::new(),
                connect_timeout_millis: 1500,
                read_timeout_millis: 3000,
            },
            smtp_host: String::new(),
            smtp_username: String::new(),
            smtp_password: String::new(),
            smtp_timeout_sec: 60u16,
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

pub(crate) fn init_global() -> Result<GlobalSettings> {
    let settings = GlobalSettings::default();
    db::write(TABLE, SETTINGS_KEY, &settings)?;
    let format = time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]]")
        .expect("Invalid format description");
    let t = time::OffsetDateTime::now_utc();
    let t_str = t.format(&format).map_err(|e| Error::TimeFormatError(e))?;
    db::write(TABLE, "db_init_time", &t_str)?;
    db::write(TABLE, "version", &String::from(server::VERSION))?;
    Ok(settings)
}

pub(crate) fn init(robot_id: &str) -> Result<Settings> {
    let settings = Settings::default();
    db::write(TABLE, robot_id, &settings)?;
    Ok(settings)
}

pub(crate) fn get_global_settings() -> Result<Option<GlobalSettings>> {
    db::query(TABLE, SETTINGS_KEY)
}

pub(crate) async fn rest_get_global_settings() -> impl IntoResponse {
    to_res(get_global_settings())
}

pub(crate) fn get_settings(robot_id: &str) -> Result<Option<Settings>> {
    db::query(TABLE, robot_id)
}

pub(crate) async fn get(Query(q): Query<RobotQuery>) -> impl IntoResponse {
    to_res::<Option<Settings>>(get_settings(&q.robot_id))
}

pub(crate) async fn save(
    Query(q): Query<RobotQuery>,
    Json(data): Json<Settings>,
) -> impl IntoResponse {
    to_res(save_settings(&q.robot_id, &data))
}

pub(crate) fn save_global_settings(data: &GlobalSettings) -> Result<()> {
    let addr = format!("{}:{}", data.ip, data.port);
    let _: SocketAddr = addr.parse().map_err(|_| {
        log::error!("Saving invalid listen IP: {}", &addr);
        Error::ErrorWithMessage(String::from("lang.settings.invalidIp"))
    })?;
    db::write(TABLE, SETTINGS_KEY, &data)
}

pub(crate) async fn rest_save_global_settings(
    Json(data): Json<GlobalSettings>,
) -> impl IntoResponse {
    to_res(save_global_settings(&data))
}

pub(crate) fn save_settings(robot_id: &str, data: &Settings) -> Result<()> {
    if let completion::TextGenerationProvider::HuggingFace(m) =
        &data.text_generation_provider.provider
    {
        if let Err(e) = completion::replace_model_cache(robot_id, &m) {
            log::warn!("Hugging face model files incorrect. Err: {:?}", &e);
        }
    }

    if let embedding::SentenceEmbeddingProvider::HuggingFace(m) =
        &data.sentence_embedding_provider.provider
    {
        match crate::ai::huggingface::load_bert_model_files(&m.get_info().repository) {
            Ok(m) => embedding::replace_model_cache(robot_id, m),
            Err(e) => {
                log::warn!("Hugging face model files incorrect. Err: {:?}", &e);
            }
        }
    }
    db::write(TABLE, robot_id, &data)
}

pub(crate) async fn smtp_test(Json(settings): Json<Settings>) -> impl IntoResponse {
    to_res(check_smtp_settings(&settings))
}

pub(crate) fn check_smtp_settings(settings: &Settings) -> Result<bool> {
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::SmtpTransport;
    let creds = Credentials::new(
        settings.smtp_username.to_owned(),
        settings.smtp_password.to_owned(),
    );

    let mailer = SmtpTransport::relay(&settings.smtp_host)?
        .credentials(creds)
        .timeout(Some(core::time::Duration::from_secs(
            settings.smtp_timeout_sec as u64,
        )))
        .build();

    Ok(mailer.test_connection()?)
}

pub(crate) async fn download_model_files(Json(m): Json<HuggingFaceModel>) -> impl IntoResponse {
    let global_settings = get_global_settings();
    if global_settings.is_err() {
        return to_res(Err(Error::ErrorWithMessage(String::from(
            "Load global settings failed.",
        ))));
    }
    let global_settings = global_settings.unwrap();
    if global_settings.is_none() {
        return to_res(Err(Error::ErrorWithMessage(String::from(
            "Global settings not found.",
        ))));
    }
    let global_settings = global_settings.unwrap();
    tokio::spawn(async move {
        match huggingface::download_hf_models(
            &m.get_info(),
            &global_settings.hf_model_download.access_token,
            global_settings.hf_model_download.connect_timeout_millis as u64,
            global_settings.hf_model_download.read_timeout_millis as u64,
        )
        .await
        {
            Ok(_) => log::info!("All model files download successfully."),
            Err(e) => log::error!("Model file downloaded failed, err: {:?}", &e),
        }
        // if let Some(s) = huggingface::DOWNLOAD_STATUS.get() {
        //     if let Ok(mut v) = s.lock() {
        //         v.downloading = false;
        //     }
        // }
    });
    to_res(Ok(()))
}

pub(crate) async fn download_model_progress() -> impl IntoResponse {
    let r = huggingface::get_download_status();
    to_res(Ok(r))
}

pub(crate) async fn check_model_files(bytes: Bytes) -> impl IntoResponse {
    match serde_json::from_slice::<Vec<HuggingFaceModel>>(bytes.as_ref()) {
        Ok(repositories) => {
            let mut map = Map::new();
            for model in repositories.iter() {
                let info = model.get_info();
                let r = match huggingface::check_model_files(&info) {
                    Ok(r) => r,
                    Err(e) => {
                        log::warn!(
                            "Hugging face model {} files incorrect. Err: {:?}",
                            info.repository,
                            &e
                        );
                        false
                    }
                };
                map.insert(model.to_string(), Value::from(r));
            }
            to_res(Ok(map))
        }
        Err(e) => to_res(Err(Error::ErrorWithMessage(format!(
            "Invalid request body, err {:?}",
            &e
        )))),
    }
}
