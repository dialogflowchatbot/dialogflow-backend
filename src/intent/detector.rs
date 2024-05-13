use std::collections::VecDeque;
use std::sync::OnceLock;
use std::vec::Vec;

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
        let mut search_vector: Option<Vector> = None;
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
            let db = match Database::open("data/intent_embeddings") {
                Ok(db) => db,
                Err(e) => {
                    log::error!("Failed open database {}", &e);
                    continue;
                }
            };
            let collection = match db.get_collection(&i.id) {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed open collection {}", &e);
                    continue;
                },
            };
            if search_vector.is_none() {
                if let Some(mut op)= embedding(s)? {
                    search_vector = Some(op.pop().unwrap().into());
                }
            }
            if search_vector.is_some() {
                let results = collection.search(search_vector.as_ref().unwrap(), 5)?;
                for r in results.iter() {
                    log::info!("r.distance={}", r.distance);
                    if r.distance >= 0.9 {
                        return Ok(Some(i.name.clone()));
                    }
                }
            }
        }
    }
    Ok(None)
}

static EMBEDDING_MODEL: OnceLock<Option<fastembed::TextEmbedding>> = OnceLock::new();

pub(crate) fn embedding(s: &str) -> Result<Option<Vec<Vec<f32>>>> {
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
        return Ok(Some(embeddings));
    }
    Ok(None)
}

pub(crate) fn save_intent_embedding(intent_id: &str, s: &str) -> Result<()> {
    let embeddings = embedding(s)?;
    if embeddings.is_none() {
        return Ok(());
    }
    let vectors: Vec<Vector> = embeddings.unwrap().iter().map(|v| v.into()).collect();
    let records: Vec<Record> = vectors.iter().map(|v| Record::new(v, &"".into())).collect();
    let mut config = Config::default();
    config.distance = Distance::Cosine;
    let collection = Collection::build(&config, &records)?;
    let mut db = Database::open("data/intent_embeddings")?;
    db.save_collection(intent_id, &collection)?;
    Ok(())
}

