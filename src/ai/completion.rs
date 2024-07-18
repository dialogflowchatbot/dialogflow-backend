use core::time::Duration;

// use crossbeam_channel::Sender;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::sync::mpsc::Sender;

use crate::ai::huggingface::{HuggingFaceModel, HuggingFaceModelType};
use crate::man::settings;
use crate::result::{Error, Result};

pub(crate) const TEMPERATURE: f64 = 0.7;
pub(crate) const REPEAT_PENALTY: f32 = 1.1;
pub(crate) const REPEAT_LAST_N: usize = 64;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "id", content = "model")]
pub(crate) enum TextGenerationProvider {
    HuggingFace(HuggingFaceModel),
    OpenAI(String),
    Ollama(String),
}

#[derive(Deserialize, Serialize)]
pub(in crate::ai) struct Prompt {
    pub(in crate::ai) role: String,
    pub(in crate::ai) content: String,
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

pub(crate) async fn completion(
    robot_id: &str,
    prompt: &str,
    sender: &Sender<String>,
) -> Result<()> {
    if let Some(settings) = settings::get_settings(robot_id)? {
        log::info!("{:?}", &settings.text_generation_provider.provider);
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
                .await?;
                Ok(())
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
                .await?;
                Ok(())
            }
        }
    } else {
        Err(Error::ErrorWithMessage(format!(
            "Can NOT retrieve settings from robot_id: {robot_id}"
        )))
    }
}

#[macro_export]
macro_rules! sse_send (
    ($sender: expr, $message: expr) => ({
        // println!("sse_send0");
        if !$sender.is_closed() {
            // println!("sse_send1");
            let sender = $sender.clone();
            // tokio::spawn(async move {
            //     log::info!("sse_send {}",&$message);
            //     if let Err(e) = sender.send($message).await {
            //         log::warn!("Failed sending LLM result, err: {:?}", &e);
            //     }
            // });
            tokio::task::spawn_blocking(move || {
                if let Err(e) = sender.blocking_send($message) {
                    log::warn!("Failed sending LLM result, err: {:?}", &e);
                }
            });
        }
    });
);

// pub(in crate::ai) fn send(sender: &Sender<String>, message: String) -> Result<()> {
//     let sender = sender.clone();
//     if let Err(e) = sender.try_send(message) {
//         match e {
//             tokio::sync::mpsc::error::TrySendError::Full(m) => Ok(sender.blocking_send(m)?),
//             tokio::sync::mpsc::error::TrySendError::Closed(_) => Err(e.into()),
//         }
//     } else {
//         Ok(())
//     }
// }

async fn huggingface(
    robot_id: &str,
    m: &HuggingFaceModel,
    prompt: &str,
    sample_len: usize,
    sender: &Sender<String>,
) -> Result<()> {
    let info = m.get_info();
    // log::info!("model_type={:?}", &info.model_type);
    let new_prompt = info.convert_prompt(prompt)?;
    match info.model_type {
        HuggingFaceModelType::Gemma => {
            super::gemma::gen_text(robot_id, &info, prompt, sample_len, Some(0.5), sender)
        }
        HuggingFaceModelType::Llama => super::llama::gen_text(
            robot_id,
            &info,
            &new_prompt,
            sample_len,
            Some(25),
            Some(0.5),
            sender,
        ),
        HuggingFaceModelType::Phi3 => {
            super::phi3::gen_text(robot_id, &info, prompt, sample_len, Some(0.5), sender)
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
    sender: &Sender<String>,
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
    sender: &Sender<String>,
) -> Result<()> {
    let prompts: Vec<Prompt> = serde_json::from_str(s)?;
    let mut prompt = String::with_capacity(32);
    for p in prompts.iter() {
        if p.role.eq("user") {
            prompt.push_str(&p.content);
            break;
        }
    }
    if prompt.is_empty() {
        return Ok(());
    }
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(connect_timeout_millis.into()))
        .read_timeout(Duration::from_millis(read_timeout_millis.into()))
        .build()?;
    let mut map = Map::new();
    map.insert(String::from("prompt"), Value::String(prompt));
    map.insert(String::from("model"), Value::String(String::from(m)));
    map.insert(String::from("stream"), Value::Bool(true));

    let mut num_predict = Map::new();
    num_predict.insert(String::from("num_predict"), Value::from(sample_len));

    map.insert(String::from("options"), Value::from(num_predict));
    let obj = Value::Object(map);
    let body = serde_json::to_string(&obj)?;
    log::info!("Request Ollama body {}", &body);
    let req = client.post(u).body(body);
    let mut stream = req.send().await?.bytes_stream();
    while let Some(item) = stream.next().await {
        let chunk = item?;
        let v: Value = serde_json::from_slice(chunk.as_ref())?;
        if let Some(res) = v.get("response") {
            if res.is_string() {
                if let Some(s) = res.as_str() {
                    let m = String::from(s);
                    log::info!("Ollama push {}", &m);
                    sse_send!(sender, m);
                }
            }
        }
    }
    Ok(())
}
