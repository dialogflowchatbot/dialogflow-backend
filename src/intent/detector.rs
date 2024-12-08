use regex::Regex;

use super::dto::{Intent, IntentDetail};
use super::phrase;
use crate::ai::embedding::embedding;
use crate::db;
use crate::db_executor;
use crate::result::{Error, Result};

pub(crate) async fn detect(robot_id: &str, s: &str) -> Result<Option<String>> {
    // let now = std::time::Instant::now();
    let op: Option<Vec<Intent>> = db_executor!(
        db::query,
        robot_id,
        super::crud::TABLE_SUFFIX,
        super::crud::INTENT_LIST_KEY
    )?;
    if op.is_none() {
        return Ok(None);
    }
    let r = op.unwrap();
    let mut empty_phrase = true;
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
            empty_phrase = i.phrase_num < 1;
        }
    }
    if empty_phrase {
        return Ok(None);
    }
    let embedding = embedding(robot_id, s).await?;
    if embedding.0.is_empty() {
        return Ok(None);
    }
    // log::info!("Generate embedding cost {:?}", now.elapsed());
    // let s = format!("{:?}", &embedding);
    // let regex = regex::Regex::new(r"\s").unwrap();
    // log::info!("detect embedding {}", regex.replace_all(&s, ""));
    // let now = std::time::Instant::now();
    let search_vector: Vec<f32> = embedding.0.into();
    let similarity_threshold = embedding.1 as f64;
    let mut result = phrase::search(robot_id, &search_vector).await?;
    // log::info!("Searching vector took {:?}", now.elapsed());
    if !result.is_empty() {
        if let Some(record) = result.get_mut(0) {
            log::info!("Record distance: {}", record.1);
            if (1f64 - record.1) >= similarity_threshold {
                let s = std::mem::replace(&mut record.0, String::new());
                return Ok(Some(s));
            }
        }
    }
    Ok(None)
}

/*
pub(crate) async fn save_intent_embedding(
    robot_id: &str,
    intent_id: &str,
    intent_name: &str,
    s: &str,
) -> Result<i64> {
    let embedding = embedding(robot_id, s).await?;
    if embedding.0.is_empty() {
        let err = format!("{s} embedding data is empty");
        log::warn!("{}", &err);
        return Err(Error::ErrorWithMessage(err));
    }
    log::info!("embedding.0.len() = {}", embedding.0.len());
    let id = phrase::add(robot_id, intent_id, intent_name, &embedding.0).await?;
    Ok(id)
}

pub(crate) async fn save_intent_embeddings(
    robot_id: &str,
    intent_id: &str,
    intent_name: &str,
    array: Vec<&str>,
) -> Result<()> {
    phrase::remove_by_intent_id(robot_id, intent_id).await?;
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
    // if embeddings.is_empty() {
    //     return Err(Error::ErrorWithMessage(String::from(
    //         "No embeddings were generated.",
    //     )));
    // }
    if !embeddings.is_empty() {
        phrase::batch_add(robot_id, intent_id, intent_name, &embeddings).await?;
    }
    Ok(())
}
*/

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
