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

static EMBEDDING_MODEL: OnceLock<Mutex<Option<fastembed::TextEmbedding>>> = OnceLock::new();

#[derive(Deserialize, Serialize)]
pub(crate) enum HuggingFaceModel {
    AllMiniLML6V2,
    ParaphraseMLMiniLML12V2,
    ParaphraseMLMpnetBaseV2,
    BgeSmallEnV1_5,
    BgeBaseEnV1_5,
    BgeLargeEnV1_5,
    BgeM3,
    NomicEmbedTextV1_5,
    MultilingualE5Small,
    MultilingualE5Base,
    MultilingualE5Large,
    MxbaiEmbedLargeV1,
}

pub(crate) struct HuggingFaceModelInfo {
    pub(crate) orig_repo: &'static str,
    repository: &'static str,
    model_files: Vec<&'static str>,
    dimenssions: u32,
}

impl HuggingFaceModel {
    pub(crate) fn get_info(&self) -> HuggingFaceModelInfo {
        match self {
            HuggingFaceModel::AllMiniLML6V2 => HuggingFaceModelInfo {
                orig_repo: "sentence-transformers/all-MiniLM-L6-v2",
                repository: "Qdrant/all-MiniLM-L6-v2-onnx",
                model_files: vec!["model.onnx"],
                dimenssions: 384,
            },
            HuggingFaceModel::ParaphraseMLMiniLML12V2 => HuggingFaceModelInfo {
                orig_repo: "sentence-transformers/paraphrase-MiniLM-L12-v2",
                repository: "Xenova/paraphrase-multilingual-MiniLM-L12-v2",
                model_files: vec!["onnx/model.onnx"],
                dimenssions: 384,
            },
            HuggingFaceModel::ParaphraseMLMpnetBaseV2 => HuggingFaceModelInfo {
                orig_repo: "sentence-transformers/paraphrase-multilingual-mpnet-base-v2",
                repository: "Xenova/paraphrase-multilingual-mpnet-base-v2",
                model_files: vec!["onnx/model.onnx"],
                dimenssions: 768,
            },
            HuggingFaceModel::BgeSmallEnV1_5 => HuggingFaceModelInfo {
                orig_repo: "BAAI/bge-small-en-v1.5",
                repository: "Xenova/bge-small-en-v1.5",
                model_files: vec!["onnx/model.onnx"],
                dimenssions: 384,
            },
            HuggingFaceModel::BgeBaseEnV1_5 => HuggingFaceModelInfo {
                orig_repo: "BAAI/bge-base-en-v1.5",
                repository: "Xenova/bge-base-en-v1.5",
                model_files: vec!["onnx/model.onnx"],
                dimenssions: 768,
            },
            HuggingFaceModel::BgeLargeEnV1_5 => HuggingFaceModelInfo {
                orig_repo: "BAAI/bge-large-en-v1.5",
                repository: "Xenova/bge-large-en-v1.5",
                model_files: vec!["onnx/model.onnx"],
                dimenssions: 1024,
            },
            HuggingFaceModel::BgeM3 => HuggingFaceModelInfo {
                orig_repo: "BAAI/bge-m3",
                repository: "BAAI/bge-m3",
                model_files: vec!["onnx/model.onnx", "onnx/model.onnx_data"],
                dimenssions: 1024,
            },
            HuggingFaceModel::NomicEmbedTextV1_5 => HuggingFaceModelInfo {
                orig_repo: "nomic-ai/nomic-embed-text-v1.5",
                repository: "nomic-ai/nomic-embed-text-v1.5",
                model_files: vec!["onnx/model.onnx"],
                dimenssions: 768,
            },
            HuggingFaceModel::MultilingualE5Small => HuggingFaceModelInfo {
                orig_repo: "intfloat/multilingual-e5-small",
                repository: "intfloat/multilingual-e5-small",
                model_files: vec!["onnx/model.onnx"],
                dimenssions: 384,
            },
            HuggingFaceModel::MultilingualE5Base => HuggingFaceModelInfo {
                orig_repo: "intfloat/multilingual-e5-base",
                repository: "intfloat/multilingual-e5-base",
                model_files: vec!["onnx/model.onnx"],
                dimenssions: 768,
            },
            HuggingFaceModel::MultilingualE5Large => HuggingFaceModelInfo {
                orig_repo: "intfloat/multilingual-e5-large",
                repository: "Qdrant/multilingual-e5-large-onnx",
                model_files: vec!["model.onnx"],
                dimenssions: 1024,
            },
            HuggingFaceModel::MxbaiEmbedLargeV1 => HuggingFaceModelInfo {
                orig_repo: "mixedbread-ai/mxbai-embed-large-v1",
                repository: "mixedbread-ai/mxbai-embed-large-v1",
                model_files: vec!["onnx/model.onnx"],
                dimenssions: 1024,
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
    let mut files = vec![
        "tokenizer.json",
        "config.json",
        "special_tokens_map.json",
        "tokenizer_config.json",
    ];
    files.extend_from_slice(&info.model_files);
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
                v.url = String::from(f);
            }
        }
        let res = client.get(&u).query(&[("download", "true")]).send().await?;
        let total_size = res.content_length().unwrap();
        // println!("Downloading {f}, total size {total_size}");
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
                    // log::info!("Downloaded {new}");
                    v.downloaded_len = new;
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn load_model_files(repository: &str) -> Result<fastembed::TextEmbedding> {
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

pub(crate) fn replace_model_cache(c: fastembed::TextEmbedding) {
    if let Some(lock) = EMBEDDING_MODEL.get() {
        if let Ok(mut cache) = lock.lock() {
            cache.replace(c);
        }
    }
}

fn hugging_face(info: &HuggingFaceModelInfo, s: &str) -> Result<Vec<f32>> {
    let lock = EMBEDDING_MODEL.get_or_init(|| Mutex::new(None));
    let mut model = match lock.lock() {
        Ok(l) => l,
        Err(e) => {
            log::warn!("{:#?}", &e);
            e.into_inner()
        }
    };
    let m = if model.is_none() {
        let loaded_model = load_model_files(&info.orig_repo)?;
        model.insert(loaded_model)
    } else {
        model.as_ref().unwrap()
    };
    let mut embeddings = m.embed(vec![s], None)?;
    if embeddings.is_empty() {
        return Err(Error::ErrorWithMessage(String::from(
            "Embedding data was empty.",
        )));
    }
    return Ok(embeddings.remove(0));
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
