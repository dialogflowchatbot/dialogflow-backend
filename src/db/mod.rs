use std::borrow::{Borrow, Cow};
use std::vec::Vec;

use once_cell::sync::Lazy;
use redb::ReadableTableMetadata;
use redb::{Database, ReadableTable, TableDefinition};

use crate::external::http::crud as http;
use crate::flow::mainflow::crud as mainflow;
use crate::flow::rt::context;
use crate::intent::crud as intent;
use crate::man::settings::{self, GlobalSettings};
use crate::result::{Error, Result};
use crate::robot::crud as robot;
use crate::variable::crud as variable;
use crate::web::server;

// const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("flow");
// const RUNTIME_NODE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("runtimeNodes");
const TABLE_FILE_NAME: &str = "./data/flow.db";

#[macro_export]
macro_rules! db_executor (
    ($func: expr, $robot_id: expr, $suffix: expr, $($bind: expr),*) => ({
        let table_name = format!("{}{}", $robot_id, $suffix);
        let table: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new(&table_name);
        $func(table $(,($bind))*)
    });
);

pub(crate) static DB: Lazy<Database> = Lazy::new(|| {
    let data_folder = std::path::Path::new("./data");
    if !data_folder.exists() {
        std::fs::create_dir(data_folder).expect("Create data directory failed.");
    }
    let path = std::path::Path::new(TABLE_FILE_NAME);
    if path.exists() {
        Database::open(TABLE_FILE_NAME).expect("Open database failed.")
    } else {
        let db = Database::create(TABLE_FILE_NAME).expect("Create database failed.");
        // let write_txn = db.begin_write().expect("Starting transaction failed");
        // let _ = write_txn.open_table(TABLE).expect("Opening table failed");
        // // let _ = write_txn.open_table(RUNTIME_NODE_TABLE)?;
        // write_txn.commit().expect("Commiting transaction failed");
        db
    }
});

pub(crate) fn init() -> Result<GlobalSettings> {
    let is_en = *server::IS_EN;

    // Settings
    settings::init_table()?;
    mainflow::init_default_names(is_en)?;
    if settings::exists()? {
        return Ok(settings::get_global_settings()?.unwrap());
    }
    let settings = settings::init_global()?;
    let robot_id = robot::init(is_en)?;
    settings::init(&robot_id)?;
    // 意图
    intent::init(&robot_id, is_en)?;
    // 变量
    variable::init(&robot_id, is_en)?;
    // 主流程
    mainflow::init(&robot_id)?;
    // 流程上下文
    context::init()?;
    // Http 接口
    http::init(&robot_id)?;
    Ok(settings)
}

pub(crate) fn init_table<K, V>(table: TableDefinition<K, V>) -> Result<()>
where
    K: redb::Key,
    for<'b> V: redb::Value<SelfType<'b> = &'b [u8]>,
{
    let write_txn = DB.begin_write().expect("Starting transaction failed");
    let _ = write_txn.open_table(table).expect("Opening table failed");
    write_txn.commit().expect("Commiting transaction failed");
    Ok(())
}

// https://users.rust-lang.org/t/requesting-help-with-a-lifetime-problem-using-redb/98553
// https://doc.rust-lang.org/nomicon/hrtb.html
// https://users.rust-lang.org/t/implementation-is-not-general-enough/57433/4
pub(crate) fn query<'a, K, V, KEY, D>(table: TableDefinition<K, V>, key: KEY) -> Result<Option<D>>
where
    K: redb::Key,
    for<'b> V: redb::Value<SelfType<'b> = &'b [u8]>,
    D: serde::de::DeserializeOwned,
    KEY: Borrow<&'a str> + std::borrow::Borrow<<K as redb::Value>::SelfType<'a>>,
{
    let read = DB.begin_read()?;
    let table = read.open_table(table)?;
    let r = table.get(key)?;
    if let Some(d) = r {
        let s: D = serde_json::from_slice(d.value())?;
        Ok(Some(s))
    } else {
        Ok(None)
    }
}

pub(crate) fn get_all<K, V, D>(table: TableDefinition<K, V>) -> Result<Vec<D>>
where
    K: redb::Key,
    for<'b> V: redb::Value<SelfType<'b> = &'b [u8]>,
    D: serde::de::DeserializeOwned,
{
    let read = DB.begin_read()?;
    let table = read.open_table(table)?;
    let mut v: Vec<D> = Vec::with_capacity(20);
    for range in table.iter()? {
        if let Ok((_key, value)) = range {
            let s: D = serde_json::from_slice(value.value())?;
            v.push(s)
        }
    }
    Ok(v)
}

pub(crate) fn range<'a, K, V, KR, D>(
    table: TableDefinition<K, V>,
    range: impl std::ops::RangeBounds<KR> + 'a,
) -> Result<Vec<D>>
where
    K: redb::Key,
    for<'b> V: redb::Value<SelfType<'b> = &'b [u8]>,
    KR: Borrow<K::SelfType<'a>> + 'a,
    D: serde::de::DeserializeOwned,
{
    let read = DB.begin_read()?;
    let table = read.open_table(table)?;
    let r = table.range(range)?;
    let mut v: Vec<D> = Vec::with_capacity(10);
    for d in r {
        let (_key, value) = d?;
        let s: D = serde_json::from_slice(value.value())?;
        v.push(s)
    }
    Ok(v)
}

pub(crate) fn count<K, V>(table: TableDefinition<K, V>) -> Result<u64>
where
    K: redb::Key,
    for<'a> V: redb::Value<SelfType<'a> = &'a [u8]>,
{
    let read = DB.begin_read()?;
    let table = read.open_table(table)?;
    let l = table.len()?;
    Ok(l)
}

// key: impl for<'a> Borrow<K::SelfType<'a>>,
// key: K::SelfType<'_>,
// pub(crate) fn write<K, V, D>(table: TableDefinition<K, V>, key: &str, value: &D) -> Result<()>
// https://users.rust-lang.org/t/requesting-help-with-saving-data-into-redb-lifetime-problem/98586/7
pub(crate) fn write<V, D>(table: TableDefinition<&str, V>, key: &str, value: &D) -> Result<()>
where
    V: for<'a> redb::Value<SelfType<'a> = &'a [u8]>,
    D: serde::Serialize,
{
    match serde_json::to_vec(value) {
        Ok(r) => {
            let write_txn = DB.begin_write()?;
            {
                let mut table = write_txn.open_table(table)?;
                // 这里不能用key，是因为insert方法，限定了两个参数是一个生命周期，而r.as_str()短于key的，会编译不通过
                table.insert(key, r.as_slice())?;
            }
            write_txn.commit()?;
            Ok(())
        }
        Err(e) => Err(Error::ErrorWithMessage(format!("{:?}", e))),
    }
}

/*
pub(crate) fn read<'a, D>(key: impl Borrow<&'a str>) -> Result<Option<D>>
where
    D: serde::de::DeserializeOwned,
{
    // let db = Database::open(TABLE_FILE_NAME)?;
    let read_txn = DB.begin_read()?;
    let table = read_txn.open_table(TABLE)?;
    let record = table.get(key)?;
    if let Some(r) = record {
        let json = serde_json::from_slice(r.value())?;
        return Ok(Some(json));
    }
    Ok(None)
}

pub(crate) fn process_data<I: serde::de::DeserializeOwned, R, F: FnMut(I) -> Result<R>>(
    key: &str,
    mut f: F,
) -> Result<Option<R>> {
    let r: Option<I> = read(key)?;
    if let Some(d) = r {
        let v = f(d)?;
        return Ok(Some(v));
    }
    return Ok(None);
}

pub(crate) fn process_data<I: serde::de::DeserializeOwned, R, F: FnMut(&mut I) -> Result<R>> (
    key: &str,
    mut f: F,
) -> Result<Option<R>> {
    let r: Option<I> = read(key)?;
    if let Some(mut d) = r {
        let v = f(&mut d)?;
        return Ok(Some(v));
    }
    return Ok(None);
    // let r: Result<Option<I>> = read(key);
    // if let Ok(o) = r {
    //     if let Some(d) = o {
    //         f(d)?;
    //     }
    // }
}

pub(crate) fn save<'a, D>(key: impl Borrow<&'a str>, value: &D) -> Result<()>
where
    D: serde::Serialize,
{
    match serde_json::to_vec(value) {
        Ok(r) => {
            // let db = Database::open(TABLE_FILE_NAME)?;
            let write_txn = DB.begin_write()?;
            {
                let mut table = write_txn.open_table(TABLE)?;
                // 这里不能用key，是因为insert方法，限定了两个参数是一个生命周期，而r.as_str()短于key的，会编译不通过
                table.insert(key.borrow(), r.as_slice())?;
            }
            write_txn.commit()?;
            Ok(())
        }
        Err(e) => Err(Error::ErrorWithMessage(format!("{:?}", e))),
    }
}

pub(crate) fn save_txn<'a>(
    v: Vec<(impl Borrow<&'a str>, Box<&dyn erased_serde::Serialize>)>,
) -> Result<()> {
    // let db = Database::open(TABLE_FILE_NAME)?;
    let write_txn = DB.begin_write()?;
    let mut err: Option<Error> = None;
    {
        let mut table = write_txn.open_table(TABLE)?;
        for d in v {
            match serde_json::to_vec(&d.1) {
                Ok(r) => {
                    table.insert(d.0.borrow(), r.as_slice())?;
                }
                Err(e) => {
                    err = Some(Error::ErrorWithMessage(format!("{:?}", e)));
                    break;
                }
            };
        }
    }
    if err.is_some() {
        write_txn.abort()?;
        return Err(err.unwrap());
    }
    write_txn.commit()?;
    Ok(())
}
*/

// pub(crate) fn save_txn<V>(
pub(crate) fn save_txn(
    // v: Vec<(TableDefinition<K, V>,impl for<'a> Borrow<K::SelfType<'a>>, Box<&dyn erased_serde::Serialize>)>,
    v: Vec<(
        // TableDefinition<&str, V>,
        &str,
        &str,
        &str,
        Box<&dyn erased_serde::Serialize>,
    )>,
) -> Result<()>
// where
//     V: for<'b> redb::Value<SelfType<'b> = &'b [u8]>,
{
    // let db = Database::open(TABLE_FILE_NAME)?;
    let write_txn = DB.begin_write()?;
    let mut err: Option<Error> = None;
    {
        for d in v {
            let table_name = format!("{}{}", d.0, d.1);
            let t: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new(&table_name);
            let mut table = write_txn.open_table(t)?;
            match serde_json::to_vec(&d.2) {
                Ok(r) => {
                    table.insert(d.1, r.as_slice())?;
                }
                Err(e) => {
                    err = Some(Error::ErrorWithMessage(format!("{:?}", e)));
                    break;
                }
            };
        }
    }
    if err.is_some() {
        write_txn.abort()?;
        return Err(err.unwrap());
    }
    write_txn.commit()?;
    Ok(())
}

pub(crate) fn remove<'a, K, V, KEY>(table: redb::TableDefinition<K, V>, key: KEY) -> Result<()>
where
    K: redb::Key,
    for<'b> V: redb::Value<SelfType<'b> = &'b [u8]>,
    KEY: Borrow<&'a str> + std::borrow::Borrow<<K as redb::Value>::SelfType<'a>>,
{
    // let db = Database::open(TABLE_FILE_NAME)?;
    let write_txn = DB.begin_write()?;
    {
        let mut table = write_txn.open_table(table)?;
        table.remove(key)?;
    }
    write_txn.commit()?;
    Ok(())
}

pub(crate) fn delete_table<'a, K, V>(table: redb::TableDefinition<K, V>) -> Result<()>
where
    K: redb::Key,
    for<'b> V: redb::Value<SelfType<'b> = &'b [u8]>,
{
    // let db = Database::open(TABLE_FILE_NAME)?;
    let write_txn = DB.begin_write()?;
    {
        write_txn.delete_table(table)?;
    }
    write_txn.commit()?;
    Ok(())
}
