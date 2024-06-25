use core::time::Duration;
// use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::sync::mpsc::Sender;

use crate::ai::huggingface::{HuggingFaceModel, HuggingFaceModelType};
use crate::man::settings;
use crate::result::{Error, Result};

pub(crate) const TEMPERATURE: f64=0.8;
pub(crate) const REPEAT_PENALTY:f32=1.1;
pub(crate) const REPEAT_LAST_N:usize = 64;

#[derive(Deserialize, Serialize)]
#[serde(tag = "id", content = "model")]
pub(crate) enum TextGenerationProvider {
    HuggingFace(HuggingFaceModel),
    OpenAI(String),
    Ollama(String),
}

pub(crate) fn replace_model_cache(robot_id: &str, m: &HuggingFaceModel) -> Result<()> {
    let info = m.get_info();
    match info.model_type {
        HuggingFaceModelType::Llama => super::llama::replace_model_cache(robot_id, &info),
        HuggingFaceModelType::Gemma => super::gemma::replace_model_cache(robot_id, &info),
        HuggingFaceModelType::Phi3 => super::phi3::replace_model_cache(robot_id, &info),
        HuggingFaceModelType::Bert => Err(Error::ErrorWithMessage(format!(
            "Unsuported model type {:?}.",
            &info.model_type
        ))),
    }
}

pub(crate) async fn completion(robot_id: &str, prompt: &str, sender: Sender<String>) -> Result<()> {
    if let Some(settings) = settings::get_settings(robot_id)? {
        match settings.text_generation_provider.provider {
            TextGenerationProvider::HuggingFace(m) => {
                huggingface(
                    robot_id,
                    &m,
                    prompt,
                    settings.text_generation_provider.max_response_token_length as usize,
                    sender,
                )
                .await?;
                Ok(())
            }
            TextGenerationProvider::OpenAI(m) => {
                open_ai(
                    &m,
                    prompt,
                    settings.text_generation_provider.connect_timeout_millis,
                    settings.text_generation_provider.read_timeout_millis,
                    sender,
                )
                .await
            }
            TextGenerationProvider::Ollama(m) => {
                ollama(
                    &settings.text_generation_provider.api_url,
                    &m,
                    prompt,
                    settings.text_generation_provider.connect_timeout_millis,
                    settings.text_generation_provider.read_timeout_millis,
                    settings.text_generation_provider.max_response_token_length,
                    sender,
                )
                .await
            }
        }
    } else {
        Err(Error::ErrorWithMessage(format!(
            "Can NOT retrieve settings from robot_id: {robot_id}"
        )))
    }
}

pub(in crate::ai) fn send(sender: &Sender<String>, message: String) -> Result<()> {
    if let Err(e) = sender.try_send(message) {
        match e {
            tokio::sync::mpsc::error::TrySendError::Full(m) => Ok(sender.blocking_send(m)?),
            tokio::sync::mpsc::error::TrySendError::Closed(_) => Err(e.into()),
        }
    } else {
        Ok(())
    }
}

async fn huggingface(
    robot_id: &str,
    m: &HuggingFaceModel,
    prompt: &str,
    sample_len: usize,
    sender: Sender<String>,
) -> Result<()> {
    let info = m.get_info();
    log::info!("model_type={:?}", &info.model_type);
    let new_prompt = info.convert_prompt(prompt)?;
    match info.model_type {
        HuggingFaceModelType::Gemma => {
            super::gemma::gen_text(robot_id, &info, prompt, sample_len, None, sender)
        }
        HuggingFaceModelType::Llama => {
            super::llama::gen_text(robot_id, &info, &new_prompt, sample_len, None, None, sender)
        }
        HuggingFaceModelType::Phi3 => {
            super::phi3::gen_text(robot_id, &info, prompt, sample_len, None, sender)
        }
        HuggingFaceModelType::Bert => Err(Error::ErrorWithMessage(format!(
            "Unsuported model type {:?}.",
            &info.model_type
        ))),
    }
    // Ok(())
}

async fn open_ai(
    m: &str,
    s: &str,
    connect_timeout_millis: u16,
    read_timeout_millis: u16,
    sender: Sender<String>,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(connect_timeout_millis.into()))
        .read_timeout(Duration::from_millis(read_timeout_millis.into()))
        .build()?;
    let mut message0 = Map::new();
    message0.insert(String::from("role"), Value::from("system"));
    message0.insert(String::from("content"), Value::from("system_hint"));
    let mut message1 = Map::new();
    message1.insert(String::from("role"), Value::from("user"));
    message1.insert(String::from("content"), Value::from(s));
    let messages = Value::Array(vec![message0.into(), message1.into()]);
    let mut map = Map::new();
    map.insert(String::from("model"), Value::from(m));
    map.insert(String::from("messages"), messages);
    let obj = Value::Object(map);
    let req = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer ")
        .body(serde_json::to_string(&obj)?);
    let r = req
        // .timeout(Duration::from_millis(60000))
        .send()
        .await?
        .text()
        .await?;
    let v: Value = serde_json::from_str(&r)?;
    let mut embedding_result: Vec<f32> = Vec::with_capacity(3072);
    if let Some(d) = v["data"].as_array() {
        for item in d.iter() {
            if let Some(embedding) = item["embedding"].as_array() {
                for e in embedding.iter() {
                    if let Some(n) = e.as_number() {
                        if let Some(num) = n.as_f64() {
                            embedding_result.push(num as f32);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

async fn ollama(
    u: &str,
    m: &str,
    s: &str,
    connect_timeout_millis: u16,
    read_timeout_millis: u16,
    sample_len: u32,
    sender: Sender<String>,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(connect_timeout_millis.into()))
        .read_timeout(Duration::from_millis(read_timeout_millis.into()))
        .build()?;
    let mut map = Map::new();
    map.insert(String::from("prompt"), Value::String(String::from(s)));
    map.insert(String::from("model"), Value::String(String::from(m)));

    let mut num_predict = Map::new();
    num_predict.insert(String::from("num_predict"), Value::from(sample_len));

    map.insert(String::from("options"), Value::from(num_predict));
    let obj = Value::Object(map);
    let req = client.post(u).body(serde_json::to_string(&obj)?);
    let b = req
        // .timeout(Duration::from_millis(60000))
        .send()
        .await?
        .bytes()
        .await?;
    let v: Value = serde_json::from_slice(b.as_ref())?;
    let s = if let Some(r) = v.get("response") {
        r.as_str().unwrap_or("")
    } else {
        ""
    };
    Ok(())
}
