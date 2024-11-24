use core::time::Duration;

use std::fs::OpenOptions;
use std::path::Path;
use std::sync::OnceLock;

use axum::{
    extract::{Multipart, Query},
    response::IntoResponse,
    Json,
};
use futures_util::StreamExt;
use sqlx::{pool::PoolOptions, Row, Sqlite};

use super::dto::{QuestionAnswersPair, QuestionData};
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
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT
            questions TEXT NOT NULL,
            answer TEXT NOT NULL
        );",
        robot_id, robot_id
    );
    // log::info!("sql = {}", &sql);
    let mut stream = sqlx::raw_sql(&sql).execute_many(DATA_SOURCE.get().unwrap());
    while let Some(res) = stream.next().await {
        match res {
            Ok(_r) => log::info!("Initialized intent table"),
            Err(e) => log::error!("Create table failed, err: {:?}", e),
        }
    }
    // let dml = include_str!("../resource/sql/dml.sql");
    // if let Err(e) = sqlx::query(dml).execute(&pool).await {
    //     panic!("{:?}", e);
    // }
    Ok(())
}

pub(crate) async fn add(robot_id: &str, mut d: QuestionAnswersPair) -> Result<String> {
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
            let sql = format!("INSERT INTO {}_vec_row_id (id)VALUES(NULL)", robot_id);
            let last_insert_rowid = sqlx::query::<Sqlite>(&sql)
                .execute(DATA_SOURCE.get().unwrap())
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
                .execute(DATA_SOURCE.get().unwrap())
                .await?;
            q.vec_row_id = Some(last_insert_rowid);
        } else {
            let sql = format!("UPDATE {} SET vectors = ? WHERE = ?", robot_id);
            let vec_row_id = q.vec_row_id.unwrap();
            sqlx::query::<Sqlite>(&sql)
                .bind(serde_json::to_string(&vectors.0)?)
                .bind(vec_row_id)
                .execute(DATA_SOURCE.get().unwrap())
                .await?;
        };
    }
    let sql = format!(
        "INSERT INTO {}_qa(id, data, created_at)VALUES(?, ?, ?)",
        robot_id
    );
    sqlx::query::<Sqlite>(&sql)
        .bind(&qa_id)
        .bind(serde_json::to_string(&d)?)
        .bind(0)
        .execute(DATA_SOURCE.get().unwrap())
        .await?;
    Ok(qa_id)
}
