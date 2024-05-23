use core::time::Duration;

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};
use std::vec::Vec;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use crate::man::settings;
use crate::result::{Error, Result};

#[derive(Deserialize, Serialize)]
#[serde(tag = "id", content = "model")]
pub(crate) enum EmbeddingProvider {
    HuggingFace(HuggingFaceModel),
    OpenAI(String),
    Ollama(String),
}

pub(super) async fn embedding(s: &str) -> Result<Vec<f32>> {
    if let Some(settings) = settings::get_settings()? {
        match settings.embedding_provider.provider {
            EmbeddingProvider::HuggingFace(m) => hugging_face(&m.get_info(), s),
            EmbeddingProvider::OpenAI(m) => open_ai(&m, s).await,
            EmbeddingProvider::Ollama(m) => ollama(&m, s).await,
        }
    } else {
        Ok(vec![])
    }
}

static EMBEDDING_MODEL: OnceLock<Option<fastembed::TextEmbedding>> = OnceLock::new();

#[derive(Deserialize, Serialize)]
pub(crate) enum HuggingFaceModel {
    AllMiniLML6V2,
    BgeSmallEnV1_5,
    BgeBaseEnV1_5,
}

pub(crate) struct HuggingFaceModelInfo {
    pub(crate) orig_repo: String,
    repository: String,
    model_file: String,
    dimenssions: u32,
}

impl HuggingFaceModel {
    pub(crate) fn get_info(&self) -> HuggingFaceModelInfo {
        match self {
            HuggingFaceModel::AllMiniLML6V2 => HuggingFaceModelInfo {
                orig_repo: String::from("Qdrant/all-MiniLM-L6-v2-onnx"),
                repository: String::from("Qdrant/all-MiniLM-L6-v2-onnx"),
                model_file: String::from("model.onnx"),
                dimenssions: 384,
            },
            HuggingFaceModel::BgeSmallEnV1_5 => HuggingFaceModelInfo {
                orig_repo: todo!(),
                repository: todo!(),
                model_file: todo!(),
                dimenssions: todo!(),
            },
            HuggingFaceModel::BgeBaseEnV1_5 => HuggingFaceModelInfo {
                orig_repo: todo!(),
                repository: todo!(),
                model_file: todo!(),
                dimenssions: todo!(),
            },
        }
    }
}

const HUGGING_FACE_MODEL_ROOT: &str = "./data/hf_hub/";

#[derive(Clone, Serialize)]
pub(crate) struct DownloadStatus {
    pub(crate) downloading: bool,
    #[serde(rename = "totalLen")]
    pub(crate) total_len: u64,
    #[serde(rename = "downloadedLen")]
    pub(crate) downloaded_len: u64,
    pub(crate) url: String,
}

pub(crate) static DOWNLOAD_STATUS: OnceLock<Mutex<DownloadStatus>> = OnceLock::new();

pub(crate) fn get_download_status() -> Option<DownloadStatus> {
    if let Some(op) = DOWNLOAD_STATUS.get() {
        return match op.lock() {
            Ok(s) => Some(s.clone()),
            Err(e) => {
                log::error!("{:?}", &e);
                None
            }
        };
    }
    None
}

pub(crate) async fn download_hf_models(info: &HuggingFaceModelInfo) -> Result<()> {
    if let Ok(v) = DOWNLOAD_STATUS
        .get_or_init(|| {
            Mutex::new(DownloadStatus {
                downloading: false,
                total_len: 1,
                downloaded_len: 0,
                url: String::new(),
            })
        })
        .lock()
    {
        if v.downloading {
            return Err(Error::ErrorWithMessage(String::from(
                "Model files are downloading.",
            )));
        }
    }
    let root_path = format!("{}{}", HUGGING_FACE_MODEL_ROOT, info.orig_repo);
    tokio::fs::create_dir_all(&root_path).await?;
    let mut builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(5000))
        .read_timeout(Duration::from_millis(10000));
    if let Ok(proxy) = std::env::var("https_proxy") {
        if !proxy.is_empty() {
            log::info!("Detected proxy setting: {}", &proxy);
            builder = builder.proxy(reqwest::Proxy::https(&proxy)?)
        }
    }
    let client = builder.build()?;
    let files = vec![
        &info.model_file,
        "tokenizer.json",
        "config.json",
        "special_tokens_map.json",
        "tokenizer_config.json",
    ];
    for &f in files.iter() {
        let file_path_str = format!("{}/{}", &root_path, f);
        let file_path = std::path::Path::new(&file_path_str);
        if tokio::fs::try_exists(file_path).await? {
            continue;
        }
        let u = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            &info.repository, f
        );
        if let Some(s) = DOWNLOAD_STATUS.get() {
            if let Ok(mut v) = s.lock() {
                v.downloading = true;
                v.url = u.clone();
            }
        }
        let res = client.get(&u).query(&[("download", "true")]).send().await?;
        println!("22222222");
        let total_size = res.content_length().unwrap();
        println!("Downloading {f}, total size {total_size}");
        if let Some(s) = DOWNLOAD_STATUS.get() {
            if let Ok(mut v) = s.lock() {
                v.total_len = total_size;
            }
        }
        // let b = res.bytes().await?;
        // fs::write("./temp.file", b.as_ref()).await?;
        // let mut downloaded = 0u64;
        let mut stream = res.bytes_stream();
        let mut file = OpenOptions::new()
            .read(false)
            .write(true)
            .truncate(false)
            .create_new(true)
            .open(file_path)
            .await?;
        // let mut file = File::create("./temp.file").await?;

        while let Some(item) = stream.next().await {
            let chunk = item?;
            file.write_all(&chunk).await?;
            if let Some(s) = DOWNLOAD_STATUS.get() {
                if let Ok(mut v) = s.lock() {
                    let new = std::cmp::min(v.downloaded_len + (chunk.len() as u64), total_size);
                    log::info!("Downloaded {new}");
                    v.downloaded_len = new;
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn load_model(repository: &str) -> Result<fastembed::TextEmbedding> {
    let model_files = [
        "model.onnx",
        "tokenizer.json",
        "config.json",
        "special_tokens_map.json",
        "tokenizer_config.json",
    ];
    let mut model_file_streams = VecDeque::with_capacity(model_files.len());
    for &f in model_files.iter() {
        let file_path = format!("{}{}/{}", HUGGING_FACE_MODEL_ROOT, repository, f);
        let stream = std::fs::read(&file_path)?;
        model_file_streams.push_back(stream);
        // match std::fs::read(&file_path) {
        //     Ok(s) => model_file_streams.push_back(s),
        //     Err(e) => {
        //         log::warn!("Failed read model file {f}, err: {}, ", e);
        //         return None;
        //     }
        // };
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
    let r = fastembed::TextEmbedding::try_new_from_user_defined(config, opt)?;
    Ok(r)
}

fn hugging_face(info: &HuggingFaceModelInfo, s: &str) -> Result<Vec<f32>> {
    let model = EMBEDDING_MODEL.get_or_init(|| {
        match load_model(&info.orig_repo) {
            Ok(m) => Some(m),
            Err(e) => {
                log::error!("Failed read model files err: {:?}, ", e);
                None
            }
        }
        // if let Ok(model) = load_model(&info.repository) {
        //     Some(model)
        // } else {
        //     None
        // }
    });
    if let Some(m) = model {
        let mut embeddings = m.embed(vec![s], None)?;
        if embeddings.is_empty() {
            return Err(Error::ErrorWithMessage(String::from(
                "Embedding data was empty.",
            )));
        }
        return Ok(embeddings.remove(0));
    }
    Err(Error::ErrorWithMessage(String::from(
        "Hugging Face model files can NOT be found.",
    )))
}

async fn open_ai(m: &str, s: &str) -> Result<Vec<f32>> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(2000))
        .read_timeout(Duration::from_millis(10000))
        .build()?;
    let mut map = Map::new();
    map.insert(String::from("input"), Value::String(String::from(s)));
    map.insert(String::from("model"), Value::String(String::from(m)));
    let obj = Value::Object(map);
    let req = client
        .post("https://api.openai.com/v1/embeddings")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer ")
        .body(serde_json::to_string(&obj)?);
    let r = req
        // .timeout(Duration::from_millis(60000))
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

async fn ollama(m: &str, s: &str) -> Result<Vec<f32>> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(2000))
        .read_timeout(Duration::from_millis(10000))
        .build()?;
    let mut map = Map::new();
    map.insert(String::from("prompt"), Value::String(String::from(s)));
    map.insert(String::from("model"), Value::String(String::from(m)));
    let obj = Value::Object(map);
    let req = client
        .post("https://api.openai.com/v1/embeddings")
        .body(serde_json::to_string(&obj)?);
    let r = req
        // .timeout(Duration::from_millis(60000))
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
