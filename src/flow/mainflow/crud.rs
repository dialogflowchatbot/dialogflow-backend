use std::sync::OnceLock;

use axum::{response::IntoResponse, Json};
use once_cell::sync::Lazy;
use redb::TableDefinition;
use tokio::sync::Mutex;

use super::dto::MainFlowDetail;
use crate::db;
use crate::flow::subflow::crud as subflow;
use crate::result::{Error, Result};
use crate::web::server::to_res;

const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("mainflows");

static LOCK: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
static DEFAULT_NAMES: OnceLock<(String, String)> = OnceLock::new();

pub(crate) fn init_default_names(is_en: bool) -> Result<()> {
    let (name, subflow_name) = if is_en {
        ("The first main flow", "First sub-flow")
    } else {
        ("第一个主流程", "第一个子流程")
    };
    DEFAULT_NAMES
        .set((String::from(name), String::from(subflow_name)))
        .map_err(|_| Error::ErrorWithMessage(String::from("Dup")))
}

pub(crate) async fn init() -> Result<MainFlowDetail> {
    db::init_table(TABLE)?;
    db::init_table(subflow::TABLE)?;
    create_main_flow(&DEFAULT_NAMES.get().unwrap().0).await
}

pub(crate) async fn list() -> impl IntoResponse {
    to_res::<Vec<MainFlowDetail>>(db::get_all(TABLE))
}

pub(crate) async fn new(Json(data): Json<MainFlowDetail>) -> impl IntoResponse {
    to_res::<MainFlowDetail>(create_main_flow(&data.name).await)
}

async fn create_main_flow(name: &str) -> Result<MainFlowDetail> {
    let _ = LOCK.lock().await;
    let count = db::count(TABLE)?;
    let mut buffer = itoa::Buffer::new();
    let count = buffer.format(count + 1);
    let id = format!("{}{}", count, scru128::new_string());
    let main_flow = MainFlowDetail {
        id,
        name: String::from(name),
        enabled: true,
    };
    db::write(TABLE, main_flow.id.as_str(), &main_flow)?;
    subflow::new_subflow(&main_flow.id, &DEFAULT_NAMES.get().unwrap().1).await?;
    Ok(main_flow)
}

pub(crate) async fn save(Json(data): Json<MainFlowDetail>) -> impl IntoResponse {
    let main_flow = MainFlowDetail {
        id: data.id.clone(),
        name: data.name.clone(),
        enabled: data.enabled,
    };
    to_res(db::write(TABLE, &data.id, &main_flow))
}

pub(crate) async fn delete(Json(data): Json<MainFlowDetail>) -> impl IntoResponse {
    let main_flow_id = data.id.as_str();
    match crate::flow::rt::crud::remove_runtime_nodes(main_flow_id) {
        Ok(_) => to_res(db::remove(TABLE, main_flow_id)),
        Err(e) => to_res(Err(e)),
    }
}
