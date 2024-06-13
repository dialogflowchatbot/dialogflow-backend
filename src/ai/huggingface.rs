use core::time::Duration;
use std::sync::{Mutex, OnceLock};

use candle::{DType, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, HiddenAct, DTYPE};
use candle_transformers::models::phi3::{Config as Phi3Config, Model as Phi3};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokenizers::{AddedToken, PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};
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
    Phi3Mini4kInstruct,
}

pub(crate) struct HuggingFaceModelInfo {
    pub(crate) repository: &'static str,
    mirror: &'static str,
    model_files: Vec<&'static str>,
    model_index_file: &'static str,
    tokenizer_filename: &'static str,
    dimenssions: u32,
}

fn get_common_model_files() -> Vec<&'static str> {
    vec![
        "model.safetensors",
        "tokenizer.json",
        "config.json",
        "special_tokens_map.json",
        "tokenizer_config.json",
    ]
}

impl HuggingFaceModel {
    pub(crate) fn get_info(&self) -> HuggingFaceModelInfo {
        match self {
            HuggingFaceModel::AllMiniLML6V2 => HuggingFaceModelInfo {
                repository: "sentence-transformers/all-MiniLM-L6-v2",
                mirror: "sentence-transformers/all-MiniLM-L6-v2",
                model_files: get_common_model_files(),
                model_index_file: "",
                tokenizer_filename: "tokenizer.json",
                dimenssions: 384,
            },
            HuggingFaceModel::ParaphraseMLMiniLML12V2 => HuggingFaceModelInfo {
                repository: "sentence-transformers/paraphrase-MiniLM-L12-v2",
                mirror: "sentence-transformers/paraphrase-MiniLM-L12-v2",
                model_files: get_common_model_files(),
                model_index_file: "",
                tokenizer_filename: "tokenizer.json",
                dimenssions: 384,
            },
            HuggingFaceModel::ParaphraseMLMpnetBaseV2 => HuggingFaceModelInfo {
                repository: "sentence-transformers/paraphrase-multilingual-mpnet-base-v2",
                mirror: "sentence-transformers/paraphrase-multilingual-mpnet-base-v2",
                model_files: get_common_model_files(),
                model_index_file: "",
                tokenizer_filename: "tokenizer.json",
                dimenssions: 768,
            },
            HuggingFaceModel::BgeSmallEnV1_5 => HuggingFaceModelInfo {
                repository: "BAAI/bge-small-en-v1.5",
                mirror: "BAAI/bge-small-en-v1.5",
                model_files: get_common_model_files(),
                model_index_file: "",
                tokenizer_filename: "tokenizer.json",
                dimenssions: 384,
            },
            HuggingFaceModel::BgeBaseEnV1_5 => HuggingFaceModelInfo {
                repository: "BAAI/bge-base-en-v1.5",
                mirror: "BAAI/bge-base-en-v1.5",
                model_files: get_common_model_files(),
                model_index_file: "",
                tokenizer_filename: "tokenizer.json",
                dimenssions: 768,
            },
            HuggingFaceModel::BgeLargeEnV1_5 => HuggingFaceModelInfo {
                repository: "BAAI/bge-large-en-v1.5",
                mirror: "BAAI/bge-large-en-v1.5",
                model_files: get_common_model_files(),
                model_index_file: "",
                tokenizer_filename: "tokenizer.json",
                dimenssions: 1024,
            },
            HuggingFaceModel::BgeM3 => HuggingFaceModelInfo {
                repository: "BAAI/bge-m3",
                mirror: "BAAI/bge-m3",
                model_files: vec!["onnx/model.onnx", "onnx/model.onnx_data"],
                model_index_file: "",
                tokenizer_filename: "tokenizer.json",
                dimenssions: 1024,
            },
            HuggingFaceModel::NomicEmbedTextV1_5 => HuggingFaceModelInfo {
                repository: "nomic-ai/nomic-embed-text-v1.5",
                mirror: "nomic-ai/nomic-embed-text-v1.5",
                model_files: get_common_model_files(),
                model_index_file: "",
                tokenizer_filename: "tokenizer.json",
                dimenssions: 768,
            },
            HuggingFaceModel::MultilingualE5Small => HuggingFaceModelInfo {
                repository: "intfloat/multilingual-e5-small",
                mirror: "intfloat/multilingual-e5-small",
                model_files: get_common_model_files(),
                model_index_file: "",
                tokenizer_filename: "tokenizer.json",
                dimenssions: 384,
            },
            HuggingFaceModel::MultilingualE5Base => HuggingFaceModelInfo {
                repository: "intfloat/multilingual-e5-base",
                mirror: "intfloat/multilingual-e5-base",
                model_files: get_common_model_files(),
                model_index_file: "",
                tokenizer_filename: "tokenizer.json",
                dimenssions: 768,
            },
            HuggingFaceModel::MultilingualE5Large => HuggingFaceModelInfo {
                repository: "intfloat/multilingual-e5-large",
                mirror: "intfloat/multilingual-e5-large",
                model_files: get_common_model_files(),
                model_index_file: "",
                tokenizer_filename: "tokenizer.json",
                dimenssions: 1024,
            },
            HuggingFaceModel::MxbaiEmbedLargeV1 => HuggingFaceModelInfo {
                repository: "mixedbread-ai/mxbai-embed-large-v1",
                mirror: "mixedbread-ai/mxbai-embed-large-v1",
                model_files: get_common_model_files(),
                model_index_file: "",
                tokenizer_filename: "tokenizer.json",
                dimenssions: 1024,
            },
            HuggingFaceModel::Phi3Mini4kInstruct => HuggingFaceModelInfo {
                repository: "microsoft/Phi-3-mini-4k-instruct",
                mirror: "microsoft/Phi-3-mini-4k-instruct",
                model_files: vec![""],
                model_index_file: "model.safetensors.index.json",
                tokenizer_filename: "tokenizer.json",
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
    let files = if info.model_index_file.is_empty() {
        // files.extend_from_slice(&info.model_files);
        info.model_files
            .iter()
            .map(|&s| String::from(s))
            .collect::<Vec<_>>()
    } else {
        let model_index_file = construct_model_file_path(&info.mirror, &info.model_index_file);
        let path = std::path::Path::new(&model_index_file);
        if !path.exists() {
            download_hf_file(&client, info, &root_path, &info.model_index_file).await?;
        }
        load_safetensors(&info.mirror, &info.model_index_file)?
    };
    for f in files.iter() {
        download_hf_file(&client, info, &root_path, f).await?;
        // let file_path_str = format!("{}/{}", &root_path, f);
        // let file_path = std::path::Path::new(&file_path_str);
        // if tokio::fs::try_exists(file_path).await? {
        //     continue;
        // }
        // let u = format!("https://huggingface.co/{}/resolve/main/{}", &info.mirror, f);
        // if let Some(s) = DOWNLOAD_STATUS.get() {
        //     if let Ok(mut v) = s.lock() {
        //         v.downloading = true;
        //         v.url = String::from(f);
        //     }
        // }
        // let res = client.get(&u).query(&[("download", "true")]).send().await?;
        // let total_size = res.content_length().unwrap();
        // // println!("Downloading {f}, total size {total_size}");
        // if let Some(s) = DOWNLOAD_STATUS.get() {
        //     if let Ok(mut v) = s.lock() {
        //         v.total_len = total_size;
        //     }
        // }
        // // let b = res.bytes().await?;
        // // fs::write("./temp.file", b.as_ref()).await?;
        // // let mut downloaded = 0u64;
        // let mut stream = res.bytes_stream();
        // let mut file = OpenOptions::new()
        //     .read(false)
        //     .write(true)
        //     .truncate(false)
        //     .create_new(true)
        //     .open(file_path)
        //     .await?;
        // // let mut file = File::create("./temp.file").await?;

        // while let Some(item) = stream.next().await {
        //     let chunk = item?;
        //     file.write_all(&chunk).await?;
        //     if let Some(s) = DOWNLOAD_STATUS.get() {
        //         if let Ok(mut v) = s.lock() {
        //             let new = std::cmp::min(v.downloaded_len + (chunk.len() as u64), total_size);
        //             // log::info!("Downloaded {new}");
        //             v.downloaded_len = new;
        //         }
        //     }
        // }
    }
    Ok(())
}

async fn download_hf_file(
    client: &reqwest::Client,
    info: &HuggingFaceModelInfo,
    root_path: &str,
    f: &str,
) -> Result<()> {
    let file_path_str = format!("{}/{}", root_path, f);
    let file_path = std::path::Path::new(&file_path_str);
    if tokio::fs::try_exists(file_path).await? {
        return Ok(());
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
    Ok(())
}

pub(super) fn construct_model_file_path(mirror: &str, f: &str) -> String {
    format!("{}{}/{}", HUGGING_FACE_MODEL_ROOT, mirror, f)
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

enum HfModel {
    Bert((BertModel, Tokenizer)),
    Phi3(Phi3),
}

fn load_safetensors(mirror: &str, json_file: &str) -> Result<Vec<String>> {
    let json_file = construct_model_file_path(mirror, json_file);
    let json_file = std::fs::File::open(json_file)?;
    let json: serde_json::Value =
        serde_json::from_reader(&json_file).map_err(candle::Error::wrap)?;
    let weight_map = match json.get("weight_map") {
        None => {
            return Err(Error::ErrorWithMessage(format!(
                "no weight map in {json_file:?}"
            )))
        }
        Some(serde_json::Value::Object(map)) => map,
        Some(_) => {
            return Err(Error::ErrorWithMessage(format!(
                "weight map in {json_file:?} is not a map"
            )))
        }
    };
    let mut safetensors_files = std::collections::HashSet::new();
    for value in weight_map.values() {
        if let Some(file) = value.as_str() {
            safetensors_files.insert(file.to_string());
        }
    }
    Ok(Vec::from_iter(safetensors_files))
}

fn load_phi3_model_files(info: &HuggingFaceModelInfo) -> Result<HfModel> {
    let device = device()?;
    let dtype = if device.is_cuda() {
        DType::BF16
    } else {
        DType::F32
    };
    let filenames = load_safetensors(&info.mirror, &info.model_index_file)?
        .iter()
        .map(|v| std::path::PathBuf::from(construct_model_file_path(&info.mirror, v)))
        .collect::<Vec<_>>();
    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&filenames, dtype, &device)? };
    let config_filename = construct_model_file_path(&info.mirror, "config.json");
    let config = std::fs::read_to_string(config_filename)?;
    let config: Phi3Config = serde_json::from_str(&config)?;
    let phi3 = Phi3::new(&config, vb)?;
    Ok(HfModel::Phi3(phi3))
}
