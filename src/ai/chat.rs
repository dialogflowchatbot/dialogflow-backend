use core::time::Duration;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::sync::mpsc::Sender;

use crate::ai::huggingface::{HuggingFaceModel, LoadedHuggingFaceModel};
use crate::man::settings;
use crate::result::{Error, Result};

static LOADED_MODELS: LazyLock<Mutex<HashMap<String, LoadedHuggingFaceModel>>> =
    LazyLock::new(|| Mutex::new(HashMap::with_capacity(32)));

pub(crate) enum ResultReceiver<'r> {
    SseSender(&'r Sender<String>),
    StrBuf(&'r mut String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "id", content = "model")]
pub(crate) enum ChatProvider {
    HuggingFace(HuggingFaceModel),
    OpenAI(String),
    Ollama(String),
}

pub(crate) fn replace_model_cache(robot_id: &str, m: &HuggingFaceModel) -> Result<()> {
    let m = LoadedHuggingFaceModel::load(m)?;
    let mut r = LOADED_MODELS.lock()?;
    r.insert(String::from(robot_id), m);
    Ok(())
}

pub(crate) async fn chat(
    robot_id: &str,
    prompt: &str,
    result_receiver: ResultReceiver<'_>,
) -> Result<()> {
    if let Some(settings) = settings::get_settings(robot_id)? {
        // log::info!("{:?}", &settings.chat_provider.provider);
        match settings.chat_provider.provider {
            ChatProvider::HuggingFace(m) => {
                huggingface(
                    robot_id,
                    &m,
                    prompt,
                    settings.chat_provider.max_response_token_length as usize,
                    result_receiver,
                )
                .await?;
                Ok(())
            }
            ChatProvider::OpenAI(m) => {
                open_ai(
                    &m,
                    prompt,
                    settings.text_generation_provider.connect_timeout_millis,
                    settings.text_generation_provider.read_timeout_millis,
                    &settings.text_generation_provider.proxy_url,
                    result_receiver,
                )
                .await?;
                Ok(())
            }
            ChatProvider::Ollama(m) => {
                ollama(
                    &settings.text_generation_provider.api_url,
                    &m,
                    prompt,
                    settings.text_generation_provider.connect_timeout_millis,
                    settings.text_generation_provider.read_timeout_millis,
                    &settings.text_generation_provider.proxy_url,
                    settings.text_generation_provider.max_response_token_length,
                    result_receiver,
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

async fn huggingface(
    robot_id: &str,
    m: &HuggingFaceModel,
    prompt: &str,
    sample_len: usize,
    mut result_receiver: ResultReceiver<'_>,
) -> Result<()> {
    let info = m.get_info();
    // log::info!("model_type={:?}", &info.model_type);
    let new_prompt = info.convert_prompt(prompt)?;
    let mut model = LOADED_MODELS.lock().unwrap_or_else(|e| {
        log::warn!("{:#?}", &e);
        e.into_inner()
    });
    if !model.contains_key(robot_id) {
        let r = LoadedHuggingFaceModel::load(m)?;
        model.insert(String::from(robot_id), r);
    };
    let loaded_model = model.get(robot_id).unwrap();
    match loaded_model {
        LoadedHuggingFaceModel::Gemma(m) => super::gemma::gen_text(
            &m.0,
            &m.1,
            &m.2,
            prompt,
            sample_len,
            Some(0.5),
            &mut result_receiver,
        ),
        LoadedHuggingFaceModel::Llama(m) => super::llama::gen_text(
            &m.0,
            &m.1,
            &m.2,
            &m.3,
            &m.4,
            &new_prompt,
            sample_len,
            Some(25),
            Some(0.5),
            &mut result_receiver,
        ),
        LoadedHuggingFaceModel::Phi3(m) => super::phi3::gen_text(
            &m.0,
            &m.1,
            &m.2,
            prompt,
            sample_len,
            Some(0.5),
            &mut result_receiver,
        ),
        LoadedHuggingFaceModel::Bert(_m) => Err(Error::ErrorWithMessage(format!(
            "Unsuported model type {:?}.",
            &info.model_type
        ))),
    }
    // Ok(())
}

async fn open_ai(
    m: &str,
    s: &str,
    connect_timeout_millis: u32,
    read_timeout_millis: u32,
    proxy_url: &str,
    result_receiver: ResultReceiver<'_>,
) -> Result<()> {
    let mut client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(connect_timeout_millis.into()))
        .read_timeout(Duration::from_millis(read_timeout_millis.into()));
    if proxy_url.is_empty() {
        client = client.no_proxy();
    } else {
        let proxy = reqwest::Proxy::http(proxy_url)?;
        client = client.proxy(proxy);
    }
    let client = client.build()?;
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
    let stream = match result_receiver {
        ResultReceiver::SseSender(_) => true,
        ResultReceiver::StrBuf(_) => false,
    };
    map.insert(String::from("stream"), Value::Bool(stream));
    let obj = Value::Object(map);
    let req = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer ")
        .body(serde_json::to_string(&obj)?);
    let res = req.send().await?;
    match result_receiver {
        ResultReceiver::SseSender(sender) => {
            let mut stream = res.bytes_stream();
            while let Some(item) = stream.next().await {
                let chunk = item?;
                let v: Value = serde_json::from_slice(chunk.as_ref())?;
                if let Some(choices) = v.get("choices") {
                    if choices.is_array() {
                        if let Some(choices) = choices.as_array() {
                            if !choices.is_empty() {
                                if let Some(item) = choices.get(0) {
                                    if let Some(delta) = item.get("delta") {
                                        if let Some(content) = delta.get("content") {
                                            if content.is_string() {
                                                if let Some(s) = content.as_str() {
                                                    let m = String::from(s);
                                                    log::info!("OpenAI push {}", &m);
                                                    crate::sse_send!(sender, m);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        ResultReceiver::StrBuf(sb) => {
            let v: Value = serde_json::from_slice(res.text().await?.as_ref())?;
            if let Some(choices) = v.get("choices") {
                if choices.is_array() {
                    if let Some(choices) = choices.as_array() {
                        if !choices.is_empty() {
                            if let Some(item) = choices.get(0) {
                                if let Some(message) = item.get("message") {
                                    if let Some(content) = message.get("content") {
                                        if content.is_string() {
                                            if let Some(s) = content.as_str() {
                                                log::info!("OpenAI push {}", s);
                                                sb.push_str(s);
                                            }
                                        }
                                    }
                                }
                            }
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
    connect_timeout_millis: u32,
    read_timeout_millis: u32,
    proxy_url: &str,
    sample_len: u32,
    result_receiver: ResultReceiver<'_>,
) -> Result<()> {
    log::info!("s1=|{}|", s);
    let prompts: Vec<super::completion::Prompt> = serde_json::from_str(s)?;
    log::info!("s2=|{}|", s);
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
    let mut client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(connect_timeout_millis.into()))
        .read_timeout(Duration::from_millis(read_timeout_millis.into()));
    if proxy_url.is_empty() {
        client = client.no_proxy();
    } else {
        let proxy = reqwest::Proxy::http(proxy_url)?;
        client = client.proxy(proxy);
    }
    let client = client.build()?;
    let mut map = Map::new();
    map.insert(String::from("prompt"), Value::String(prompt));
    map.insert(String::from("model"), Value::String(String::from(m)));
    let stream = match result_receiver {
        ResultReceiver::SseSender(_) => true,
        ResultReceiver::StrBuf(_) => false,
    };
    map.insert(String::from("stream"), Value::Bool(stream));

    let mut num_predict = Map::new();
    num_predict.insert(String::from("num_predict"), Value::from(sample_len));

    map.insert(String::from("options"), Value::from(num_predict));
    let obj = Value::Object(map);
    let body = serde_json::to_string(&obj)?;
    log::info!("Request Ollama body {}", &body);
    let req = client.post(u).body(body);
    let res = req.send().await?;
    match result_receiver {
        ResultReceiver::SseSender(sender) => {
            let mut stream = res.bytes_stream();
            while let Some(item) = stream.next().await {
                let chunk = item?;
                let v: Value = serde_json::from_slice(chunk.as_ref())?;
                if let Some(res) = v.get("response") {
                    if res.is_string() {
                        if let Some(s) = res.as_str() {
                            let m = String::from(s);
                            log::info!("Ollama push {}", &m);
                            crate::sse_send!(sender, m);
                        }
                    }
                }
            }
        }
        ResultReceiver::StrBuf(sb) => {
            let v: Value = serde_json::from_slice(res.bytes().await?.as_ref())?;
            if let Some(res) = v.get("response") {
                if res.is_string() {
                    if let Some(s) = res.as_str() {
                        log::info!("Ollama push {}", s);
                        sb.push_str(s);
                    }
                }
            }
        }
    }
    Ok(())
}
