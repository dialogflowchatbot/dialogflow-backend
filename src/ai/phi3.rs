use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use candle::{DType, Device, IndexOp, Tensor};
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::phi3::Model;
use tokenizers::Tokenizer;

use super::huggingface::{device, load_phi3_model_files, HuggingFaceModelInfo};
use crate::result::Result;

static TEXT_GENERATION_MODEL: OnceLock<Mutex<HashMap<String, (Model, Tokenizer)>>> =
    OnceLock::new();

pub(super) fn replace_model_cache(robot_id: &str, info: &HuggingFaceModelInfo) -> Result<()> {
    let device = device()?;
    let c = load_phi3_model_files(info, &device)?;
    if let Some(lock) = TEXT_GENERATION_MODEL.get() {
        if let Ok(mut cache) = lock.lock() {
            cache.insert(String::from(robot_id), c);
        }
    }
    Ok(())
}
