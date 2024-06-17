use std::collections::HashMap;
use std::io::Write;
use std::sync::{Mutex, OnceLock};

use candle::{DType, Tensor};
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::gemma::Model as GemmaModel;
use frand::Rand;
use tokenizers::Tokenizer;

use super::huggingface::{device, load_gemma_model_files, HuggingFaceModelInfo};
use crate::result::{Error, Result};

static TEXT_GENERATION_MODEL: OnceLock<Mutex<HashMap<String, (GemmaModel, Tokenizer)>>> =
    OnceLock::new();

pub(super) fn replace_model_cache(robot_id: &str, info: &HuggingFaceModelInfo) -> Result<()> {
    let device = device()?;
    let c = load_gemma_model_files(info, &device)?;
    if let Some(lock) = TEXT_GENERATION_MODEL.get() {
        if let Ok(mut cache) = lock.lock() {
            cache.insert(String::from(robot_id), c);
        }
    }
    Ok(())
}

pub(super) fn gen_text(
    robot_id: &str,
    info: &HuggingFaceModelInfo,
    prompt: &str,
    sample_len: usize,
    top_p: Option<f64>,
) -> Result<()> {
    let device = device()?;
    let lock = TEXT_GENERATION_MODEL.get_or_init(|| Mutex::new(HashMap::with_capacity(32)));
    let mut model = lock.lock().unwrap_or_else(|e| {
        log::warn!("{:#?}", &e);
        e.into_inner()
    });
    if !model.contains_key(robot_id) {
        let r = load_gemma_model_files(info, &device)?;
        model.insert(String::from(robot_id), r);
    };
    let (model, tokenizer) = model.get_mut(robot_id).unwrap();
    let mut tokens = match tokenizer.encode(prompt, true) {
        Ok(t) => t.get_ids().to_vec(),
        Err(e) => return Err(Error::ErrorWithMessage(format!("{}", &e))),
    };
    let mut tokenizer = super::token_output_stream::TokenOutputStream::new(tokenizer.clone());
    let eos_token = match tokenizer.get_token("<eos>") {
        Some(token) => token,
        None => {
            return Err(Error::ErrorWithMessage(String::from(
                "cannot find the <eos> token",
            )))
        }
    };
    let mut generated_tokens = 0usize;
    let repeat_penalty = 1.1f32;
    let repeat_last_n = 64usize;
    let start_gen = std::time::Instant::now();
    for index in 0..sample_len {
        let context_size = if index > 0 { 1 } else { tokens.len() };
        let start_pos = tokens.len().saturating_sub(context_size);
        let ctxt = &tokens[start_pos..];
        let input = Tensor::new(ctxt, &device)?.unsqueeze(0)?;
        let logits = model.forward(&input, start_pos)?;
        let logits = logits.squeeze(0)?.squeeze(0)?.to_dtype(DType::F32)?;
        let logits = if repeat_penalty == 1. {
            logits
        } else {
            let start_at = tokens.len().saturating_sub(repeat_last_n);
            candle_transformers::utils::apply_repeat_penalty(
                &logits,
                repeat_penalty,
                &tokens[start_at..],
            )?
        };

        let mut rng = Rand::new();
        let temperature = 0.8f64;
        let mut logits_processor = LogitsProcessor::new(rng.gen::<u64>(), Some(temperature), top_p);
        let next_token = logits_processor.sample(&logits)?;
        tokens.push(next_token);
        generated_tokens += 1;
        if next_token == eos_token {
            break;
        }
        if let Some(t) = tokenizer.next_token(next_token)? {
            print!("{t}");
            std::io::stdout().flush()?;
        }
    }
    let dt = start_gen.elapsed();
    if let Some(rest) = tokenizer.decode_rest()? {
        print!("{rest}");
    }
    std::io::stdout().flush()?;
    println!(
        "\n{generated_tokens} tokens generated ({:.2} token/s)",
        generated_tokens as f64 / dt.as_secs_f64(),
    );
    Ok(())
}
