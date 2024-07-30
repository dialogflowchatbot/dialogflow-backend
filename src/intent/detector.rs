use oasysdb::prelude::*;
use regex::Regex;

use super::dto::{Intent, IntentDetail};
use crate::ai::embedding::embedding;
use crate::db;
use crate::db_executor;
use crate::result::{Error, Result};

pub(crate) const SAVING_PATH_ROOT: &str = "./data/intentev/";

pub(crate) async fn detect(robot_id: &str, s: &str) -> Result<Option<String>> {
    // let now = std::time::Instant::now();
    let op: Option<Vec<Intent>> = db_executor!(
        db::query,
        robot_id,
        super::crud::TABLE_SUFFIX,
        super::crud::INTENT_LIST_KEY
    )?;
    // println!("inner intent detect {:?}", now.elapsed());
    if let Some(r) = op {
        let mut search_vector: Option<Vector> = None;
        let mut similarity_threshold = 0u8;
        for i in r.iter() {
            let r: Option<IntentDetail> = db_executor!(
                db::query,
                robot_id,
                super::crud::TABLE_SUFFIX,
                i.id.as_str()
            )?;
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
            let db = match Database::open(&format!("{}{}", SAVING_PATH_ROOT, robot_id)) {
                Ok(db) => db,
                Err(e) => {
                    log::error!("Failed open database {}", &e);
                    continue;
                }
            };
            let collection = match db.get_collection(&i.id) {
                Ok(c) => c,
                Err(e) => {
                    if !is_col_not_found_err(&e) {
                        log::warn!("Failed open collection {}", &e);
                    }
                    continue;
                }
            };
            if search_vector.is_none() {
                let embedding = embedding(robot_id, s).await?;
                // let s = format!("{:?}", &embedding);
                // let regex = regex::Regex::new(r"\s").unwrap();
                // log::info!("detect embedding {}", regex.replace_all(&s, ""));
                search_vector = Some(embedding.0.into());
                similarity_threshold = embedding.1;
            }
            if search_vector.is_some() {
                let results = collection.search(search_vector.as_ref().unwrap(), 5)?;
                println!("{}", results.len());
                for r in results.iter() {
                    log::info!("r.distance={}", r.distance);
                    if 100u8 - (r.distance * 100f32) as u8 >= similarity_threshold {
                        return Ok(Some(i.name.clone()));
                    }
                }
            }
        }
    }
    Ok(None)
}

fn is_col_not_found_err(e: &oasysdb::prelude::Error) -> bool {
    e.kind == ErrorKind::DatabaseError
        && e.message
            .eq(oasysdb::prelude::Error::collection_not_found().message())
}

pub(crate) async fn save_intent_embedding(
    robot_id: &str,
    intent_id: &str,
    s: &str,
) -> Result<usize> {
    let embedding = embedding(robot_id, s).await?;
    if embedding.0.is_empty() {
        let err = format!("{s} embedding data is empty");
        log::warn!("{}", &err);
        return Err(Error::ErrorWithMessage(err));
    }
    // log::info!("save embedding {:#?}", &embedding);
    let mut db = Database::open(&format!("{}{}", SAVING_PATH_ROOT, robot_id))?;
    let mut collection = match db.get_collection(intent_id) {
        Ok(c) => c,
        Err(e) => {
            if is_col_not_found_err(&e) {
                let mut config = Config::default();
                config.distance = Distance::Cosine;
                let mut collection = Collection::new(&config);
                collection.set_dimension(embedding.0.len())?;
                collection
            } else {
                return Err(e.into());
            }
        }
    };
    // let records = Record::many_random(128, 5);
    // log::info!("Gened {}", records.get(0).unwrap().vector.0.get(0).unwrap());
    let vector: Vector = embedding.0.into();
    let record: Record = Record::new(&vector, &"".into());
    let r = collection.insert(&record)?;
    // let collection = Collection::build(&config, &records)?;
    db.save_collection(intent_id, &collection)?;
    db.flush()?;
    Ok(r.to_usize())
}

pub(crate) async fn save_intent_embeddings(
    robot_id: &str,
    intent_id: &str,
    array: Vec<&str>,
) -> Result<()> {
    delete_all_embeddings(robot_id, intent_id)?;
    let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(array.len());
    for &s in array.iter() {
        let embedding = embedding(robot_id, s).await?;
        if embedding.0.is_empty() {
            let err = format!("{s} embedding data is empty");
            log::warn!("{}", &err);
        } else {
            embeddings.push(embedding.0);
        }
    }
    if embeddings.is_empty() {
        return Err(Error::ErrorWithMessage(String::from(
            "No embeddings were generated.",
        )));
    }
    let mut db = Database::open(&format!("{}{}", SAVING_PATH_ROOT, robot_id))?;
    // log::info!("{:#?}", &embedding);
    // let records = Record::many_random(128, 5);
    // log::info!("Gened {}", records.get(0).unwrap().vector.0.get(0).unwrap());
    let vectors: Vec<Vector> = embeddings.iter().map(|d| d.into()).collect();
    let records: Vec<Record> = vectors.iter().map(|v| Record::new(v, &"".into())).collect();
    let mut config = Config::default();
    config.distance = Distance::Cosine;
    let collection = Collection::build(&config, &records).unwrap();
    log::info!("New collection demension is {}", collection.dimension());
    db.save_collection(intent_id, &collection)?;
    db.flush()?;
    Ok(())
}

pub(crate) fn delete_intent_embedding(robot_id: &str, intent_id: &str, id: usize) -> Result<()> {
    let mut db = Database::open(&format!("{}{}", SAVING_PATH_ROOT, robot_id))?;
    let mut collection = db.get_collection(intent_id)?;
    collection.delete(&id.into())?;
    db.save_collection(intent_id, &collection)?;
    Ok(())
}

pub(crate) fn delete_all_embeddings(robot_id: &str, intent_id: &str) -> Result<()> {
    let mut db = Database::open(&format!("{}{}", SAVING_PATH_ROOT, robot_id))?;
    if let Err(e) = db.delete_collection(intent_id) {
        if !is_col_not_found_err(&e) {
            return Err(e.into());
        }
    }
    Ok(())
}

// pub(crate) async fn save_intent_embedding2(intent_id: &str, s: &str) -> Result<()> {
// let embeddings = embedding(s)?;
// if embeddings.is_none() {
//     return Ok(());
// }
// let vectors: Vec<Vector> = embeddings.unwrap().iter().map(|v| v.into()).collect();
// let records: Vec<Record> = vectors.iter().map(|v| Record::new(v, &"".into())).collect();
// let mut config = Config::default();
// config.distance = Distance::Cosine;
// let collection = Collection::build(&config, &records)?;
// let mut db = Database::open(&format!("{}{}",SAVING_PATH,robot_id))?;
// db.save_collection(intent_id, &collection)?;
// Ok(())
// }
