use oasysdb::prelude::*;
use regex::Regex;

use super::dto::{Intent, IntentDetail};
use super::embedding::embedding;
use crate::db;
use crate::result::Result;

pub(crate) async fn detect(s: &str) -> Result<Option<String>> {
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
                }
            };
            if search_vector.is_none() {
                search_vector = Some(embedding(s).await?.into());
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

pub(crate) async fn save_intent_embedding(intent_id: &str, s: &str) -> Result<()> {
    let embedding = embedding(s).await?;
    if embedding.is_empty() {
        log::warn!("{s} embedding data is empty");
        return Ok(());
    }
    let vector: Vector = embedding.into();
    let record: Record = Record::new(&vector, &"".into());
    let mut config = Config::default();
    config.distance = Distance::Cosine;
    let records = vec![record];
    let collection = Collection::build(&config, &records)?;
    let mut db = Database::open("data/intent_embeddings")?;
    db.save_collection(intent_id, &collection)?;
    Ok(())
}

pub(crate) async fn save_intent_embedding2(intent_id: &str, s: &str) -> Result<()> {
    // let embeddings = embedding(s)?;
    // if embeddings.is_none() {
    //     return Ok(());
    // }
    // let vectors: Vec<Vector> = embeddings.unwrap().iter().map(|v| v.into()).collect();
    // let records: Vec<Record> = vectors.iter().map(|v| Record::new(v, &"".into())).collect();
    // let mut config = Config::default();
    // config.distance = Distance::Cosine;
    // let collection = Collection::build(&config, &records)?;
    // let mut db = Database::open("data/intent_embeddings")?;
    // db.save_collection(intent_id, &collection)?;
    Ok(())
}
