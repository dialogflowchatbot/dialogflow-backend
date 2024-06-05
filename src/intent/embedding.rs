use core::time::Duration;

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};
use std::vec::Vec;

use candle::{Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, HiddenAct, DTYPE};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokenizers::{AddedToken, PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};
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

pub(super) async fn embedding(robot_id: &str, s: &str) -> Result<Vec<f32>> {
    if let Some(settings) = settings::get_settings(robot_id)? {
        match settings.embedding_provider.provider {
            EmbeddingProvider::HuggingFace(m) => hugging_face(&m.get_info(), s),
            EmbeddingProvider::OpenAI(m) => {
                open_ai(
                    &m,
                    s,
                    settings.embedding_provider.connect_timeout_millis,
                    settings.embedding_provider.read_timeout_millis,
                )
                .await
            }
            EmbeddingProvider::Ollama(m) => {
                ollama(
                    &m,
                    s,
                    settings.embedding_provider.connect_timeout_millis,
                    settings.embedding_provider.read_timeout_millis,
                )
                .await
            }
        }
    } else {
        Ok(vec![])
    }
}

static EMBEDDING_MODEL: OnceLock<Mutex<Option<(BertModel, Tokenizer)>>> = OnceLock::new();

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

fn device() -> Result<Device> {
    if candle::utils::cuda_is_available() {
        Ok(Device::new_cuda(0)?)
    } else if candle::utils::metal_is_available() {
        Ok(Device::new_metal(0)?)
    } else {
        Ok(Device::Cpu)
    }
}

fn construct_model_file_path(mirror: &str, f: &str) -> String {
    format!("{}{}/{}", HUGGING_FACE_MODEL_ROOT, mirror, f)
}

// type TokenizerImpl = tokenizers::TokenizerImpl<
//     tokenizers::ModelWrapper,
//     tokenizers::NormalizerWrapper,
//     tokenizers::PreTokenizerWrapper,
//     tokenizers::PostProcessorWrapper,
//     tokenizers::DecoderWrapper,
// >;

fn set_tokenizer_config(
    mirror: &str,
    mut tokenizer: Tokenizer,
    pad_token_id: u32,
) -> Result<Tokenizer> {
    let f = construct_model_file_path(mirror, "tokenizer_config.json");
    let p = std::path::Path::new(&f);
    let t = if p.exists() {
        let j: serde_json::Value = serde_json::from_slice(std::fs::read(&f)?.as_slice())?;
        let model_max_length = j["model_max_length"]
            .as_f64()
            .expect("Error reading model_max_length from tokenizer_config.json")
            as f32;
        let max_length = 8192.min(model_max_length as usize);
        let pad_token = j["pad_token"]
            .as_str()
            .expect("Error reading pad_token from tokenier_config.json")
            .into();
        // log::info!("p1 {}", tokenizer.get_padding().unwrap().pad_token);
        // log::info!("t1 {}", tokenizer.get_truncation().unwrap().max_length);
        tokenizer
            .with_padding(Some(PaddingParams {
                strategy: PaddingStrategy::BatchLongest,
                pad_token,
                pad_id: pad_token_id,
                ..Default::default()
            }))
            .with_truncation(Some(TruncationParams {
                max_length,
                ..Default::default()
            }))
    } else {
        tokenizer.with_padding(None).with_truncation(None)
    };
    let t = match t {
        Ok(t) => t.clone().into(),
        Err(e) => {
            log::warn!("{:?}", &e);
            tokenizer
        }
    };

    Ok(t)
    // log::info!("p2 {}", tokenizer.get_padding().unwrap().pad_token);
    // log::info!("t2 {}", tokenizer.get_truncation().unwrap().max_length);
}

fn set_special_tokens_map(mirror: &str, tokenizer: &mut Tokenizer) -> Result<()> {
    let f = construct_model_file_path(mirror, "special_tokens_map.json");
    let p = std::path::Path::new(&f);
    if !p.exists() {
        return Ok(());
    }
    if let serde_json::Value::Object(root_object) =
        serde_json::from_slice(std::fs::read(&f)?.as_slice())?
    {
        for (_, value) in root_object.iter() {
            if value.is_string() {
                tokenizer.add_special_tokens(&[AddedToken {
                    content: value.as_str().unwrap().into(),
                    special: true,
                    ..Default::default()
                }]);
            } else if value.is_object() {
                tokenizer.add_special_tokens(&[AddedToken {
                    content: value["content"].as_str().unwrap().into(),
                    special: true,
                    single_word: value["single_word"].as_bool().unwrap(),
                    lstrip: value["lstrip"].as_bool().unwrap(),
                    rstrip: value["rstrip"].as_bool().unwrap(),
                    normalized: value["normalized"].as_bool().unwrap(),
                }]);
            }
        }
    }
    Ok(())
}

pub(crate) fn load_model_files(mirror: &str) -> Result<(BertModel, Tokenizer)> {
    let f = construct_model_file_path(mirror, "config.json");
    let config = std::fs::read_to_string(&f)?;
    let config: serde_json::Value = serde_json::from_str(&config)?;
    let pad_token_id = config["pad_token_id"].as_u64().unwrap_or(0) as u32;
    let config: Config = serde_json::from_value(config)?;
    let f = construct_model_file_path(mirror, "tokenizer.json");
    let tokenizer = match Tokenizer::from_file(&f) {
        Ok(t) => t,
        Err(e) => return Err(Error::ErrorWithMessage(format!("{}", &e))),
    };
    let mut tokenizer = set_tokenizer_config(mirror, tokenizer, pad_token_id)?;
    set_special_tokens_map(mirror, &mut tokenizer)?;
    let f = construct_model_file_path(mirror, "model.safetensors");
    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[&f], DTYPE, &device()?)? };
    let model = BertModel::load(vb, &config)?;
    Ok((model, tokenizer))
}

pub(crate) fn replace_model_cache(c: (BertModel, Tokenizer)) {
    if let Some(lock) = EMBEDDING_MODEL.get() {
        if let Ok(mut cache) = lock.lock() {
            cache.replace(c);
        }
    }
}

fn hugging_face(info: &HuggingFaceModelInfo, s: &str) -> Result<Vec<f32>> {
    let lock = EMBEDDING_MODEL.get_or_init(|| Mutex::new(None));
    let mut model = lock.lock().unwrap_or_else(|e| {
        log::warn!("{:#?}", &e);
        e.into_inner()
    });
    let (m, ref mut t) = if model.is_none() {
        let r = load_model_files(&info.repository)?;
        model.insert(r)
    } else {
        model.as_mut().unwrap()
    };
    // let tokenizer = match t.with_padding(None).with_truncation(None) {
    //     Ok(t) => t,
    //     Err(e) => return Err(Error::ErrorWithMessage(format!("{}", &e))),
    // };
    let tokens = match t.encode(s, true) {
        Ok(t) => t,
        Err(e) => return Err(Error::ErrorWithMessage(format!("{}", &e))),
    };
    let tokens = tokens.get_ids().to_vec();
    let token_ids = Tensor::new(&tokens[..], &m.device)?.unsqueeze(0)?;
    let token_type_ids = token_ids.zeros_like()?;
    let outputs = m.forward(&token_ids, &token_type_ids)?;
    let (_n_sentence, n_tokens, _hidden_size) = outputs.dims3()?;
    let embeddings = (outputs.sum(1)? / (n_tokens as f64))?;
    // let embeddings = embeddings.broadcast_div(&embeddings.sqr()?.sum_keepdim(1)?.sqrt()?)?;
    let r = embeddings.i(0)?.to_vec1::<f32>()?;
    Ok(r)
}

// fn tt() {
//     let prs = vec![0.1f32,0.1f32,0.1f32,0.1f32,];
//     let mut top: Vec<_> = prs.iter().enumerate().collect();
//     top.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
//     let top = top.into_iter().take(5).collect::<Vec<_>>();

//     for &(i, p) in &top {
//         println!(
//             "{:50}: {:.2}%",
//             i,
//             p * 100.0
//         );
//     }
// }

async fn open_ai(
    m: &str,
    s: &str,
    connect_timeout_millis: u16,
    read_timeout_millis: u16,
) -> Result<Vec<f32>> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(connect_timeout_millis.into()))
        .read_timeout(Duration::from_millis(read_timeout_millis.into()))
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

async fn ollama(
    m: &str,
    s: &str,
    connect_timeout_millis: u16,
    read_timeout_millis: u16,
) -> Result<Vec<f32>> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(connect_timeout_millis.into()))
        .read_timeout(Duration::from_millis(read_timeout_millis.into()))
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
