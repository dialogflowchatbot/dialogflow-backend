use core::time::Duration;

use std::fs::OpenOptions;
use std::sync::OnceLock;
use std::vec::Vec;

use futures_util::StreamExt;
use sqlx::{pool::PoolOptions, Row, Sqlite};

use super::dto::IntentPhraseData;
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
    p.join("ipev.dat")
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
        "CREATE TABLE {}_vec_row_id (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT
            -- intent_id TEXT NOT NULL,
        );",
        robot_id
    );
    // log::info!("sql = {}", &sql);
    let mut stream = sqlx::raw_sql(&sql).execute_many(DATA_SOURCE.get().unwrap());
    while let Some(res) = stream.next().await {
        match res {
            Ok(_r) => log::info!("Initialized phrase table"),
            Err(e) => log::error!("Create table failed, err: {:?}", e),
        }
    }
    // let dml = include_str!("../resource/sql/dml.sql");
    // if let Err(e) = sqlx::query(dml).execute(&pool).await {
    //     panic!("{:?}", e);
    // }
    Ok(())
}

pub(crate) async fn search(robot_id: &str, vectors: &Vec<f32>) -> Result<Vec<(String, f64)>> {
    let sql = format!(
        "SELECT intent_id, intent_name, distance FROM {} WHERE vectors MATCH ? ORDER BY distance ASC LIMIT 1",
        robot_id
    );
    let results = sqlx::query::<Sqlite>(&sql)
        .bind(serde_json::to_string(vectors)?)
        .fetch_all(DATA_SOURCE.get().unwrap())
        .await?;
    let mut names = Vec::with_capacity(results.len());
    for r in results.iter() {
        names.push((r.try_get(1)?, r.try_get(2)?));
    }
    Ok(names)
}

pub(crate) async fn add(
    robot_id: &str,
    vec_row_id: Option<i64>,
    intent_id: &str,
    intent_name: &str,
    phrase: &str,
) -> Result<i64> {
    // check_datasource(robot_id, intent_id).await?;
    let vectors = embedding(robot_id, phrase).await?;
    if vectors.0.is_empty() {
        let err = format!("{phrase} embedding data is empty");
        log::warn!("{}", &err);
        return Err(Error::ErrorWithMessage(err));
    }
    log::info!("vectors.0.len() = {}", vectors.0.len());
    let last_insert_rowid = if vec_row_id.is_none() {
        let sql = format!("INSERT INTO {}_vec_row_id (id)VALUES(NULL)", robot_id);
        let last_insert_rowid = sqlx::query::<Sqlite>(&sql)
            .execute(DATA_SOURCE.get().unwrap())
            .await?
            .last_insert_rowid();
        let sql = format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS {} USING vec0 (
                -- id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                intent_id TEXT NOT NULL,
                +intent_name TEXT NOT NULL,
                vectors float[{}]
            );
            INSERT INTO {} (rowid, intent_id, intent_name, vectors)VALUES(?, ?, ?, ?)",
            //  ON CONFLICT(rowid) DO UPDATE SET vectors = excluded.vectors;
            robot_id,
            vectors.0.len(),
            robot_id
        );
        sqlx::query::<Sqlite>(&sql)
            .bind(last_insert_rowid)
            .bind(intent_id)
            .bind(intent_name)
            .bind(serde_json::to_string(&vectors.0)?)
            .execute(DATA_SOURCE.get().unwrap())
            .await?;
        last_insert_rowid
    } else {
        let sql = format!("UPDATE {} SET vectors = ? WHERE = ?", robot_id);
        let vec_row_id = vec_row_id.unwrap();
        sqlx::query::<Sqlite>(&sql)
            .bind(serde_json::to_string(&vectors.0)?)
            .bind(vec_row_id)
            .execute(DATA_SOURCE.get().unwrap())
            .await?;
        vec_row_id
    };
    Ok(last_insert_rowid)
}

pub(crate) async fn batch_add(
    robot_id: &str,
    intent_id: &str,
    intent_name: &str,
    phrases: &Vec<IntentPhraseData>,
) -> Result<()> {
    // check_datasource(robot_id, intent_id).await?;
    for p in phrases.iter() {
        add(robot_id, Some(p.id), intent_id, intent_name, &p.phrase).await?;
    }
    Ok(())
}

pub(crate) async fn remove(robot_id: &str, id: i64) -> Result<()> {
    let sql = format!("DELETE FROM {} WHERE id = ?", robot_id);
    sqlx::query::<Sqlite>(&sql)
        .bind(id)
        .execute(DATA_SOURCE.get().unwrap())
        .await?;
    Ok(())
}

pub(crate) async fn remove_by_intent_id(robot_id: &str, intent_id: &str) -> Result<()> {
    let sql = format!("DELETE FROM {} WHERE intent_id = ?", robot_id);
    match sqlx::query::<Sqlite>(&sql)
        .bind(intent_id)
        .execute(DATA_SOURCE.get().unwrap())
        .await
    {
        Ok(_) => return Ok(()),
        Err(e) => match e {
            sqlx::Error::Database(database_error) => {
                if let Some(code) = database_error.code() {
                    if code.eq("1") {
                        return Ok::<_, Error>(());
                    }
                }
            }
            _ => return Err(e.into()),
        },
    };
    Ok(())
}

pub(crate) async fn remove_tables(robot_id: &str) -> Result<()> {
    let sql = format!("DROP TABLE {}", robot_id);
    sqlx::query::<Sqlite>(&sql)
        .execute(DATA_SOURCE.get().unwrap())
        .await?;
    Ok(())
}
