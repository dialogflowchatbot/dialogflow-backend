use core::time::Duration;

use std::fs::OpenOptions;
use std::sync::OnceLock;

use sqlx::{pool::PoolOptions, Row, Sqlite};

use crate::result::{Error, Result};

type SqliteConnPool = sqlx::Pool<Sqlite>;

// static DATA_SOURCE: OnceCell<SqliteConnPool> = OnceCell::new();
static DATA_SOURCE: OnceLock<SqliteConnPool> = OnceLock::new();
// static DATA_SOURCES: OnceLock<Mutex<HashMap<String, SqliteConnPool>>> = OnceLock::new();

pub(crate) async fn init_datasource() -> Result<()> {
    unsafe {
        libsqlite3_sys::sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    }

    match OpenOptions::new()
        .read(false)
        .write(true)
        .create(true)
        .open(get_sqlite_path().as_path())
    {
        Ok(_f) => {}
        // Err(e: ErrorKind::NotFound) => None,
        Err(e) => {
            return Err(Error::ErrorWithMessage(format!(
                "Created database file failed, err: {:?}",
                &e
            )))
        }
    };
    let pool_ops = PoolOptions::<Sqlite>::new()
        .min_connections(1)
        .max_connections(100)
        .acquire_timeout(Duration::from_secs(5))
        .test_before_acquire(true);
    let path = get_sqlite_path();
    if path.is_dir() {
        return Err(Error::ErrorWithMessage(String::from(
            "Created database file failed, there is a directory called: e.dat",
        )));
    }
    let s = format!("sqlite://{}?mode=rw", path.display());
    let conn_str = s.replace("\\", "/");
    // log::info!("Embedding database path: {}", &conn_str);
    let pool = pool_ops.connect(conn_str.as_str()).await?;
    DATA_SOURCE
        .set(pool)
        .map_err(|_| Error::ErrorWithMessage(String::from("Datasource has been set.")))
    /*
    下面这个不会打印，解决：
    1、把map换成for_each
    2、由于map是lazy的，所以需要在map后面加.collect()
     */
    /*
    match sqlite_get_list::<Tag>("SELECT id, name FROM blog_tag ORDER BY id DESC", None).await {
        Ok(tags) => tags.iter().map(|tag| {
            println!("{}", &tag.name);
            tag::put_id_name(tag.id, &tag.name);
        }),
        Err(e) => panic!("{:?}", e),
    };
    */
}

fn get_sqlite_path() -> std::path::PathBuf {
    let p = std::path::Path::new(".").join("data").join("intentev");
    if !p.exists() {
        std::fs::create_dir_all(&p).expect("Create data directory failed.");
    }
    p.join("e.dat")
}
