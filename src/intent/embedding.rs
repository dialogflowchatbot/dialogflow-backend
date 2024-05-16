use core::time::Duration;

use std::collections::VecDeque;
use std::sync::OnceLock;
use std::vec::Vec;

use hf_hub::api::tokio::ApiBuilder;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::man::settings;
use crate::result::{Error, Result};

#[derive(Deserialize, Serialize)]
pub(crate) enum EmbeddingProvider {
    HuggingFace,
    OpenAI,
    Ollama,
}

pub(super) async fn embedding(s: &str) -> Result<Vec<f32>> {
    if let Some(settings) = settings::get_settings()? {
        match settings.embedding_provider.provider {
            EmbeddingProvider::HuggingFace => hugging_face(s).await,
            EmbeddingProvider::OpenAI => open_ai(s).await,
            EmbeddingProvider::Ollama => ollama(s).await,
        }
    } else {
        Ok(vec![])
    }
}

static EMBEDDING_MODEL: OnceLock<Option<fastembed::TextEmbedding>> = OnceLock::new();

enum HuggingFaceModel {
    Small(Vec<String>),
}

// impl HuggingFaceModel {
//     pub(crate) fn get_files(&'static self) -> &'static Vec<String> {
//     }
// }

async fn hf_hub_downloader(name: &str) -> Result<()> {
    let api = ApiBuilder::new().with_progress(true).with_cache_dir("./data/hf_hub".into()).build()?;

    let _filename = api
        .model(name.to_string())
        .get("model-00001-of-00002.safetensors")
        .await?;
    Ok(())
}

async fn hugging_face2(s: &str) -> Result<Vec<Vec<f32>>> {
    let model = EMBEDDING_MODEL.get_or_init(|| {
        let model_files = [
            "D:\\work\\models\\bge-small-en-v1.5\\onnx\\model.onnx",
            "D:\\work\\models\\bge-small-en-v1.5\\tokenizer.json",
            "D:\\work\\models\\bge-small-en-v1.5\\config.json",
            "D:\\work\\models\\bge-small-en-v1.5\\special_tokens_map.json",
            "D:\\work\\models\\bge-small-en-v1.5\\tokenizer_config.json",
        ];
        let mut model_file_streams = VecDeque::with_capacity(model_files.len());
        for &f in model_files.iter() {
            match std::fs::read(f) {
                Ok(s) => model_file_streams.push_back(s),
                Err(e) => {
                    log::warn!("Failed read model file {f}, err: {}, ", e);
                    return None;
                }
            };
        }
        let config = fastembed::UserDefinedEmbeddingModel {
            onnx_file: model_file_streams.pop_front().unwrap(),
            tokenizer_files: fastembed::TokenizerFiles {
                tokenizer_file: model_file_streams.pop_front().unwrap(),
                config_file: model_file_streams.pop_front().unwrap(),
                special_tokens_map_file: model_file_streams.pop_front().unwrap(),
                tokenizer_config_file: model_file_streams.pop_front().unwrap(),
            },
        };
        let opt: fastembed::InitOptionsUserDefined = fastembed::InitOptionsUserDefined {
            execution_providers: vec![fastembed::ExecutionProviderDispatch::CPU(
                ort::CPUExecutionProvider::default(),
            )],
            max_length: 512,
        };
        if let Ok(model) = fastembed::TextEmbedding::try_new_from_user_defined(config, opt) {
            Some(model)
        } else {
            None
        }
    });
    if let Some(m) = model {
        let embeddings = m.embed(vec![s], None)?;
        return Ok(embeddings);
    }
    Err(Error::ErrorWithMessage(String::from(
        "Hugging Face model files can NOT be found.",
    )))
}

async fn hugging_face(s: &str) -> Result<Vec<f32>> {
    Ok(vec![0f32])
}

async fn open_ai(s: &str) -> Result<Vec<f32>> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(1000))
        .read_timeout(Duration::from_millis(10000))
        .build()?;
    let mut map = Map::new();
    map.insert(String::from("input"), Value::String(String::from(s)));
    map.insert(String::from("model"), Value::String(String::from(s)));
    let obj = Value::Object(map);
    let req = client
        .post("https://api.openai.com/v1/embeddings")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer ")
        .body(serde_json::to_string(&obj)?);
    let r = req
        .timeout(Duration::from_millis(10000))
        .send()
        .await?
        .text()
        .await?;
    let v: serde_json::Value = serde_json::from_str(&r)?;
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
    Ok(embedding_result)
}

async fn ollama(s: &str) -> Result<Vec<f32>> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(1000))
        .read_timeout(Duration::from_millis(10000))
        .build()?;
    let mut map = Map::new();
    map.insert(String::from("prompt"), Value::String(String::from(s)));
    map.insert(String::from("model"), Value::String(String::from(s)));
    let obj = Value::Object(map);
    let req = client
        .post("https://api.openai.com/v1/embeddings")
        .body(serde_json::to_string(&obj)?);
    let r = req
        .timeout(Duration::from_millis(10000))
        .send()
        .await?
        .text()
        .await?;
    let v: serde_json::Value = serde_json::from_str(&r)?;
    let mut embedding_result: Vec<f32> = Vec::with_capacity(3072);
    if let Some(embedding) = v["embedding"].as_array() {
        for e in embedding.iter() {
            if let Some(n) = e.as_number() {
                if let Some(num) = n.as_f64() {
                    embedding_result.push(num as f32);
                }
            }
        }
    }
    Ok(embedding_result)
}
