use std::sync::OnceLock;

use oasysdb::prelude::*;
use regex::Regex;

use super::dto::{Intent, IntentDetail};
use crate::db;
use crate::result::Result;

pub(crate) fn detect(s: &str) -> Result<Option<String>> {
    // let now = std::time::Instant::now();
    let op: Option<Vec<Intent>> = db::query(super::crud::TABLE, super::crud::INTENT_LIST_KEY)?;
    // println!("inner intent detect {:?}", now.elapsed());
    if let Some(r) = op {
        for i in r.iter() {
            let r: Option<IntentDetail> = db::query(super::crud::TABLE, i.id.as_str())?;
            if let Some(detail) = r {
                for k in detail.keywords.iter() {
                    if k.eq(s) {
                        // println!("{} {} {}", s, k, &i.name);
                        return Ok(Some(i.name.clone()));
                    }
                }
                for r in detail.regexes.iter() {
                    let re = Regex::new(r)?;
                    if re.is_match(s) {
                        return Ok(Some(i.name.clone()));
                    }
                }
            }
        }
    }
    Ok(None)
}

static EMBEDDING_MODEL: OnceLock<Option<fastembed::TextEmbedding>> = OnceLock::new();

pub(crate) fn embedding(s: &str) -> Result<()> {
    let model = EMBEDDING_MODEL.get_or_init(|| {
        let config = fastembed::UserDefinedEmbeddingModel {
            onnx_file: std::fs::read("D:\\work\\models\\bge-small-en-v1.5\\onnx\\model.onnx")
                .unwrap(),
            tokenizer_files: fastembed::TokenizerFiles {
                tokenizer_file: std::fs::read(
                    "D:\\work\\models\\bge-small-en-v1.5\\tokenizer.json",
                )
                .unwrap(),
                config_file: std::fs::read("D:\\work\\models\\bge-small-en-v1.5\\config.json")
                    .unwrap(),
                special_tokens_map_file: std::fs::read(
                    "D:\\work\\models\\bge-small-en-v1.5\\special_tokens_map.json",
                )
                .unwrap(),
                tokenizer_config_file: std::fs::read(
                    "D:\\work\\models\\bge-small-en-v1.5\\tokenizer_config.json",
                )
                .unwrap(),
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
        if let Ok(embeddings) = m.embed(vec![s], None) {
            println!("Embedding dimension: {}", embeddings[0].len());
            let vectors: Vec<Vector> = embeddings.iter().map(|v| v.into()).collect();
            let records: Vec<Record> = vectors.iter().map(|v| Record::new(v, &"".into())).collect();
            let mut config = Config::default();
            config.distance = Distance::Cosine;
            let collection = Collection::build(&config, &records)?;
            let mut db = Database::new("data/intent/id")?;
            db.save_collection("vectors", &collection)?;
        }
    }
    Ok(())
}
