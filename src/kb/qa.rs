use core::time::Duration;

use std::fs::OpenOptions;
use std::path::Path;
use std::sync::OnceLock;
use std::vec::Vec;

use axum::{
    extract::{Multipart, Query},
    response::IntoResponse,
    Json,
};
use futures_util::StreamExt;
use sqlx::{pool::PoolOptions, Row, Sqlite};

use super::dto::{QuestionAnswerData, QuestionAnswerPair, QuestionData};
use crate::ai::embedding::embedding;
use crate::result::{Error, Result};

type SqliteConnPool = sqlx::Pool<Sqlite>;

// static DATA_SOURCE: OnceCell<SqliteConnPool> = OnceCell::new();
static DATA_SOURCE: OnceLock<SqliteConnPool> = OnceLock::new();
// static DATA_SOURCES: OnceLock<Mutex<HashMap<String, SqliteConnPool>>> = OnceLock::new();

fn get_sqlite_path() -> std::path::PathBuf {
    let p = std::path::Path::new(".").join("data");
    if !p.exists() {
        std::fs::create_dir_all(&p).expect("Create data directory failed.");
    }
    p.join("kbqaev.dat")
}

pub(crate) async fn init_datasource() -> Result<()> {
    let p = get_sqlite_path();
    let pool = crate::db::init_sqlite_datasource(p.as_path()).await?;
    DATA_SOURCE
        .set(pool)
        .map_err(|_| Error::ErrorWithMessage(String::from("Datasource has been set.")))
}

pub async fn shutdown_db() {
    // let mut r = match DATA_SOURCES.lock() {
    //     Ok(l) => l,
    //     Err(e) => e.into_inner(),
    // };
    // let all_keys: Vec<String> = r.keys().map(|k| String::from(k)).collect();
    // let mut pools: Vec<SqliteConnPool> = Vec::with_capacity(all_keys.len());
    // for key in all_keys {
    //     let v = r.remove(&key).unwrap();
    //     pools.push(v);
    // }
    // tokio::task::spawn_blocking(|| async move {
    //     for p in pools.iter() {
    //         p.close().await;
    //     }
    // });
    DATA_SOURCE.get().unwrap().close().await;
}

pub(crate) async fn init_tables(robot_id: &str) -> Result<()> {
    // println!("Init database");
    // let ddl = include_str!("./embedding_ddl.sql");
    let sql = format!(
        "CREATE TABLE {}_question_vec_row_id (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT
        );
        CREATE TABLE {}_qa (
            id TEXT NOT NULL PRIMARY KEY,
            qa_data TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX idx_created_at ON {}_qa (created_at);",
        robot_id, robot_id, robot_id
    );
    // log::info!("sql = {}", &sql);
    let mut stream = sqlx::raw_sql(&sql).execute_many(DATA_SOURCE.get().unwrap());
    while let Some(res) = stream.next().await {
        match res {
            Ok(_r) => log::info!("Initialized QnA table"),
            Err(e) => log::error!("Create table failed, err: {:?}", e),
        }
    }
    // let dml = include_str!("../resource/sql/dml.sql");
    // if let Err(e) = sqlx::query(dml).execute(&pool).await {
    //     panic!("{:?}", e);
    // }
    Ok(())
}

// sqlite_trans!(
//     fn dq(    robot_id: &str,
//         mut d: QuestionAnswersPair,
//         transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,) -> Result<String> {
//         Ok(String::new())
//     }
// );

pub(crate) async fn list(robot_id: &str) -> Result<Vec<QuestionAnswerData>> {
    let sql = format!(
        "SELECT id, qa_data FROM {}_qa ORDER BY created_at DESC",
        robot_id
    );
    let results = sqlx::query::<Sqlite>(&sql)
        .fetch_all(DATA_SOURCE.get().unwrap())
        .await?;
    let mut d: Vec<QuestionAnswerData> = Vec::with_capacity(results.len());
    for r in results.iter() {
        d.push(QuestionAnswerData {
            id: r.try_get(0)?,
            qa_data: serde_json::from_str(dbg!(r.try_get(1)?))?,
        });
    }
    Ok(d)
}

pub(crate) async fn add(robot_id: &str, d: QuestionAnswerPair) -> Result<String> {
    let ds = DATA_SOURCE.get().unwrap();
    let mut transaction = ds.begin().await?;
    let r = add_inner(robot_id, d, &mut transaction).await;
    if r.is_ok() {
        transaction.commit().await?;
    } else {
        transaction.rollback().await?;
    }
    r
}

async fn add_inner(
    robot_id: &str,
    mut d: QuestionAnswerPair,
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<String> {
    let mut questions: Vec<&mut QuestionData> = Vec::with_capacity(5);
    questions.push(&mut d.question);
    if d.similar_questions.is_some() {
        let similar_questions: Vec<&mut QuestionData> =
            d.similar_questions.as_mut().unwrap().iter_mut().collect();
        questions.extend(similar_questions);
    }
    let qa_id = scru128::new_string();
    for q in questions.iter_mut() {
        let vectors = embedding(robot_id, &q.question).await?;
        if vectors.0.is_empty() {
            let err = format!("{} embedding data is empty", &q.question);
            log::warn!("{}", &err);
            return Err(Error::ErrorWithMessage(err));
        }

        log::info!("vectors.0.len() = {}", vectors.0.len());
        if q.vec_row_id.is_none() {
            let sql = format!(
                "INSERT INTO {}_question_vec_row_id (id)VALUES(NULL)",
                robot_id
            );
            let last_insert_rowid = sqlx::query::<Sqlite>(&sql)
                .execute(&mut **transaction)
                .await?
                .last_insert_rowid();
            let sql = format!(
                "CREATE VIRTUAL TABLE IF NOT EXISTS {} USING vec0 (
                +qa_id TEXT NOT NULL,
                vectors float[{}]
            );
            INSERT INTO {} (rowid, qa_id, vectors)VALUES(?, ?, ?)",
                //  ON CONFLICT(rowid) DO UPDATE SET vectors = excluded.vectors;
                robot_id,
                vectors.0.len(),
                robot_id
            );
            sqlx::query::<Sqlite>(&sql)
                .bind(last_insert_rowid)
                .bind(&qa_id)
                .bind(serde_json::to_string(&vectors.0)?)
                .execute(&mut **transaction)
                .await?;
            q.vec_row_id = Some(last_insert_rowid);
        } else {
            let sql = format!("UPDATE {} SET vectors = ? WHERE = ?", robot_id);
            let vec_row_id = q.vec_row_id.unwrap();
            sqlx::query::<Sqlite>(&sql)
                .bind(serde_json::to_string(&vectors.0)?)
                .bind(vec_row_id)
                .execute(&mut **transaction)
                .await?;
        };
    }
    let sql = format!(
        "INSERT INTO {}_qa(id, qa_data, created_at)VALUES(?, ?, unixepoch())",
        robot_id
    );
    sqlx::query::<Sqlite>(&sql)
        .bind(&qa_id)
        .bind(dbg!(serde_json::to_string(&d)?))
        .execute(&mut **transaction)
        .await?;
    Ok(qa_id)
}
