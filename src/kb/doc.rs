use core::time::Duration;

// use std::fs::File;
// use std::io::Read;
// use std::path::Path;
use std::fs::OpenOptions;
use std::sync::OnceLock;
use std::vec::Vec;

use docx_rs::read_docx;
use futures_util::StreamExt;
use sqlx::{pool::PoolOptions, Row, Sqlite};

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
    p.join("kbdocev.dat")
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
        "CREATE TABLE {}_row_id (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT
            -- intent_id TEXT NOT NULL,
        );",
        robot_id
    );
    // log::info!("sql = {}", &sql);
    let mut stream = sqlx::raw_sql(&sql).execute_many(DATA_SOURCE.get().unwrap());
    while let Some(res) = stream.next().await {
        match res {
            Ok(_r) => log::info!("Initialized doc table"),
            Err(e) => log::error!("Create table failed, err: {:?}", e),
        }
    }
    // let dml = include_str!("../resource/sql/dml.sql");
    // if let Err(e) = sqlx::query(dml).execute(&pool).await {
    //     panic!("{:?}", e);
    // }
    Ok(())
}

pub(super) fn parse_docx(buf: Vec<u8>) -> Result<String> {
    // let mut file = File::open("./numbering.docx")?;
    // let mut buf = Vec::with_capacity(3096);
    // file.read_to_end(&mut buf)?;
    let mut doc_text = String::with_capacity(3096);
    let docx = read_docx(&buf)?;
    let doc = docx.document;
    for d in doc.children.iter() {
        match d {
            docx_rs::DocumentChild::Paragraph(paragraph) => {
                for p in paragraph.children() {
                    match p {
                        docx_rs::ParagraphChild::Run(run) => {
                            for r in run.children.iter() {
                                match r {
                                    docx_rs::RunChild::Text(text) => {
                                        log::info!("Docx text={}", text.text);
                                        doc_text.push_str(&text.text);
                                        // doc_text.push('\n');
                                        // doc_text.push('\n');
                                    }
                                    docx_rs::RunChild::Sym(sym) => {
                                        log::info!("meet sym");
                                        doc_text.push_str(&sym.char);
                                    }
                                    docx_rs::RunChild::Break(_) => {
                                        log::info!("meet break");
                                        doc_text.push('\n');
                                    }
                                    docx_rs::RunChild::Tab(_) => {
                                        log::info!("meet tab");
                                        doc_text.push('\n');
                                    }
                                    _ => {}
                                }
                            }
                        }
                        docx_rs::ParagraphChild::Hyperlink(hyperlink) => {
                            log::info!("hyperlink: {:?}", hyperlink.link)
                        }
                        _ => {}
                    }
                }
            }
            docx_rs::DocumentChild::Table(_table) => {}
            docx_rs::DocumentChild::TableOfContents(_table_of_contents) => {}
            _ => {}
        }
    }
    Ok(doc_text)
}

fn parse_pdf() {}
