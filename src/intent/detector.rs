use oasysdb::prelude::*;

use crate::ai::embedding::embedding;
use crate::db::embedding as embedding_db;
use crate::result::{Error, Result};

fn get_embedding_db(robot_id: &str) -> Result<Database> {
    let db_url = crate::db::embedding::get_sqlite_path()?;
    let db = Database::open(robot_id, Some(db_url))?;
    Ok(db)
}

pub(crate) async fn detect(robot_id: &str, s: &str) -> Result<Option<String>> {
    // let now = std::time::Instant::now();
    let embedding = embedding(robot_id, s).await?;
    // let s = format!("{:?}", &embedding);
    // let regex = regex::Regex::new(r"\s").unwrap();
    // log::info!("detect embedding {}", regex.replace_all(&s, ""));
    let search_vector: Vec<f32> = embedding.0.into();
    let similarity_threshold = embedding.1;
    let result = get_embedding_db(robot_id)?.search_index(robot_id, search_vector, 1, "")?;
    // println!("inner intent detect {:?}", now.elapsed());
    if result.len() == 0 {
        if let Some(record) = result.get(0) {
            log::info!("Record distance: {}", record.distance);
            if similarity_threshold >= record.distance {
                if let Some(data) = record.data.get("intent_name") {
                    if let Some(metadata) = data {
                        if let oasysdb::types::record::DataValue::String(s) = metadata {
                            return Ok(Some(String::from(s)));
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

pub(crate) async fn save_intent_embedding(robot_id: &str, intent_id: &str, s: &str) -> Result<i64> {
    let embedding = embedding(robot_id, s).await?;
    if embedding.0.is_empty() {
        let err = format!("{s} embedding data is empty");
        log::warn!("{}", &err);
        return Err(Error::ErrorWithMessage(err));
    }
    let id = embedding_db::add(robot_id, intent_id, &embedding.0).await?;
    //todo refresh index
    get_embedding_db(robot_id)?.refresh_index(robot_id)?;
    Ok(id)
}

pub(crate) async fn save_intent_embeddings(
    robot_id: &str,
    intent_id: &str,
    array: Vec<&str>,
) -> Result<()> {
    // let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(array.len());
    for &s in array.iter() {
        let embedding = embedding(robot_id, s).await?;
        if embedding.0.is_empty() {
            let err = format!("{s} embedding data is empty");
            log::warn!("{}", &err);
        } else {
            // embeddings.push(embedding.0);
            embedding_db::add(robot_id, intent_id, &embedding.0).await?;
        }
    }
    // if embeddings.is_empty() {
    //     return Err(Error::ErrorWithMessage(String::from(
    //         "No embeddings were generated.",
    //     )));
    // }
    embedding_db::remove_by_intent_id(robot_id, intent_id).await?;
    let db = get_embedding_db(robot_id)?;
    db.delete_index(robot_id)?;
    let config = SourceConfig::new(robot_id, "id", "vectors");
    let params = ParamsIVFPQ::default();
    let algorithm = IndexAlgorithm::IVFPQ(params);
    db.create_index(robot_id, algorithm, config)?;
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
