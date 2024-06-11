use core::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::ai::embedding::SentenceEmbeddingProvider;
use crate::ai::huggingface::HuggingFaceModel;
use crate::man::settings;
use crate::result::Result;

#[derive(Deserialize, Serialize)]
#[serde(tag = "id", content = "model")]
pub(crate) enum TextGenerateProvider {
    HuggingFace(HuggingFaceModel),
    OpenAI(String),
    Ollama(String),
}

pub(crate) async fn completion(robot_id: &str, s: &str) -> Result<String> {
    if let Some(settings) = settings::get_settings(robot_id)? {
        match settings.sentence_embedding_provider.provider {
            SentenceEmbeddingProvider::HuggingFace(m) => todo!(),
            SentenceEmbeddingProvider::OpenAI(m) => {
                open_ai(
                    &m,
                    &settings.text_generate_provider.system_hint,
                    s,
                    settings.sentence_embedding_provider.connect_timeout_millis,
                    settings.sentence_embedding_provider.read_timeout_millis,
                )
                .await
            }
            SentenceEmbeddingProvider::Ollama(m) => {
                ollama(
                    &settings.text_generate_provider.api_url,
                    &m,
                    s,
                    settings.sentence_embedding_provider.connect_timeout_millis,
                    settings.sentence_embedding_provider.read_timeout_millis,
                )
                .await
            }
        }
    } else {
        Ok(String::new())
    }
}

async fn open_ai(
    m: &str,
    system_hint: &str,
    s: &str,
    connect_timeout_millis: u16,
    read_timeout_millis: u16,
) -> Result<String> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(connect_timeout_millis.into()))
        .read_timeout(Duration::from_millis(read_timeout_millis.into()))
        .build()?;
    let mut message0 = Map::new();
    message0.insert(String::from("role"), Value::from("system"));
    message0.insert(String::from("content"), Value::from(system_hint));
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
    Ok(String::new())
}

async fn ollama(
    u: &str,
    m: &str,
    s: &str,
    connect_timeout_millis: u16,
    read_timeout_millis: u16,
) -> Result<String> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(connect_timeout_millis.into()))
        .read_timeout(Duration::from_millis(read_timeout_millis.into()))
        .build()?;
    let mut map = Map::new();
    map.insert(String::from("prompt"), Value::String(String::from(s)));
    map.insert(String::from("model"), Value::String(String::from(m)));
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
    Ok(String::from(s))
}
