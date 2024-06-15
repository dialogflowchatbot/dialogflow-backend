use tokenizers::Tokenizer;

use crate::result::{Error, Result};

use super::huggingface::{load_llama_model_files, HuggingFaceModelInfo};

pub(crate) fn t(info: &HuggingFaceModelInfo, prompt: &str) -> Result<()> {
    let (model, cache, tokenizer, eos_token_id) = load_llama_model_files(info)?;
    // let mut tokens = tokenizer
    //     .encode(prompt, true)
    //     .map_err(|e| Err(Error::ErrorWithMessage(format!("{}", &e))))?
    //     .get_ids()
    //     .to_vec();
    Ok(())
}
