use core::time::Duration;
use std::sync::{Mutex, OnceLock};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use crate::result::{Error, Result};

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
    pub(crate) repository: &'static str,
    mirror: &'static str,
    model_files: Vec<&'static str>,
    dimenssions: u32,
}

impl HuggingFaceModel {
    pub(crate) fn get_info(&self) -> HuggingFaceModelInfo {
        match self {
            HuggingFaceModel::AllMiniLML6V2 => HuggingFaceModelInfo {
                repository: "sentence-transformers/all-MiniLM-L6-v2",
                mirror: "sentence-transformers/all-MiniLM-L6-v2",
                model_files: vec!["model.safetensors"],
                dimenssions: 384,
            },
            HuggingFaceModel::ParaphraseMLMiniLML12V2 => HuggingFaceModelInfo {
                repository: "sentence-transformers/paraphrase-MiniLM-L12-v2",
                mirror: "sentence-transformers/paraphrase-MiniLM-L12-v2",
                model_files: vec!["model.safetensors"],
                dimenssions: 384,
            },
            HuggingFaceModel::ParaphraseMLMpnetBaseV2 => HuggingFaceModelInfo {
                repository: "sentence-transformers/paraphrase-multilingual-mpnet-base-v2",
                mirror: "sentence-transformers/paraphrase-multilingual-mpnet-base-v2",
                model_files: vec!["model.safetensors"],
                dimenssions: 768,
            },
            HuggingFaceModel::BgeSmallEnV1_5 => HuggingFaceModelInfo {
                repository: "BAAI/bge-small-en-v1.5",
                mirror: "BAAI/bge-small-en-v1.5",
                model_files: vec!["model.safetensors"],
                dimenssions: 384,
            },
            HuggingFaceModel::BgeBaseEnV1_5 => HuggingFaceModelInfo {
                repository: "BAAI/bge-base-en-v1.5",
                mirror: "BAAI/bge-base-en-v1.5",
                model_files: vec!["model.safetensors"],
                dimenssions: 768,
            },
            HuggingFaceModel::BgeLargeEnV1_5 => HuggingFaceModelInfo {
                repository: "BAAI/bge-large-en-v1.5",
                mirror: "BAAI/bge-large-en-v1.5",
                model_files: vec!["model.safetensors"],
                dimenssions: 1024,
            },
            HuggingFaceModel::BgeM3 => HuggingFaceModelInfo {
                repository: "BAAI/bge-m3",
                mirror: "BAAI/bge-m3",
                model_files: vec!["onnx/model.onnx", "onnx/model.onnx_data"],
                dimenssions: 1024,
            },
            HuggingFaceModel::NomicEmbedTextV1_5 => HuggingFaceModelInfo {
                repository: "nomic-ai/nomic-embed-text-v1.5",
                mirror: "nomic-ai/nomic-embed-text-v1.5",
                model_files: vec!["model.safetensors"],
                dimenssions: 768,
            },
            HuggingFaceModel::MultilingualE5Small => HuggingFaceModelInfo {
                repository: "intfloat/multilingual-e5-small",
                mirror: "intfloat/multilingual-e5-small",
                model_files: vec!["model.safetensors"],
                dimenssions: 384,
            },
            HuggingFaceModel::MultilingualE5Base => HuggingFaceModelInfo {
                repository: "intfloat/multilingual-e5-base",
                mirror: "intfloat/multilingual-e5-base",
                model_files: vec!["model.safetensors"],
                dimenssions: 768,
            },
            HuggingFaceModel::MultilingualE5Large => HuggingFaceModelInfo {
                repository: "intfloat/multilingual-e5-large",
                mirror: "intfloat/multilingual-e5-large",
                model_files: vec!["model.safetensors"],
                dimenssions: 1024,
            },
            HuggingFaceModel::MxbaiEmbedLargeV1 => HuggingFaceModelInfo {
                repository: "mixedbread-ai/mxbai-embed-large-v1",
                mirror: "mixedbread-ai/mxbai-embed-large-v1",
                model_files: vec!["model.safetensors"],
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
    let root_path = format!("{}{}", HUGGING_FACE_MODEL_ROOT, info.repository);
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
        let u = format!("https://huggingface.co/{}/resolve/main/{}", &info.mirror, f);
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

pub(super) fn construct_model_file_path(mirror: &str, f: &str) -> String {
    format!("{}{}/{}", HUGGING_FACE_MODEL_ROOT, mirror, f)
}
