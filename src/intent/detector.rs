use crate::ai::embedding::embedding;
use crate::db::embedding as embedding_db;
use crate::result::{Error, Result};

pub(crate) async fn detect(robot_id: &str, s: &str) -> Result<Option<String>> {
    // let now = std::time::Instant::now();
    let embedding = embedding(robot_id, s).await?;
    // log::info!("Generate embedding cost {:?}", now.elapsed());
    // let s = format!("{:?}", &embedding);
    // let regex = regex::Regex::new(r"\s").unwrap();
    // log::info!("detect embedding {}", regex.replace_all(&s, ""));
    // let now = std::time::Instant::now();
    let search_vector: Vec<f32> = embedding.0.into();
    let similarity_threshold = embedding.1;
    let result = embedding_db::search_idx_db(robot_id, search_vector.into())?;
    // log::info!("Searching vector took {:?}", now.elapsed());
    if !result.is_empty() {
        if let Some(record) = result.get(0) {
            log::info!("Record distance: {}", record.distance);
            if (1f32 - record.distance) >= similarity_threshold {
                if let Some(data) = record.data.get("intent_id") {
                    if let Some(metadata) = data {
                        if let oasysdb::types::record::DataValue::String(s) = metadata {
                            let intent = super::crud::get_detail_by_id(robot_id, s)?;
                            return Ok(intent.map(|i| i.intent_name));
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
    Ok(id)
}

pub(crate) async fn save_intent_embeddings(
    robot_id: &str,
    intent_id: &str,
    array: Vec<&str>,
) -> Result<()> {
    embedding_db::remove_by_intent_id(robot_id, intent_id).await?;
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
        embedding_db::batch_add(robot_id, intent_id, &embeddings).await?;
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
