use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Json;

use super::dto::HttpReqInfo;
use crate::db;
use crate::result::Result;
use crate::web::server::to_res;

pub(crate) const TABLE: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("externalHttpApis");

pub(crate) fn init() -> Result<()> {
    db::init_table(TABLE)
}

pub(crate) async fn list() -> impl IntoResponse {
    let r: Result<Vec<HttpReqInfo>> = db::get_all(TABLE);
    to_res(r)
}

pub(crate) fn get_detail(id: &str) -> Result<Option<HttpReqInfo>> {
    db::query(TABLE, id)
}

pub(crate) async fn detail(Path(id): Path<String>) -> impl IntoResponse {
    let r: Result<Option<HttpReqInfo>> = get_detail(id.as_str());
    to_res(r)
}

pub(crate) async fn save(Json(mut params): Json<HttpReqInfo>) -> impl IntoResponse {
    if params.id.is_empty() || params.id.eq("new") {
        params.id = scru128::new_string();
    }
    let r = db::write(TABLE, &params.id, &params);
    to_res(r)
}

pub(crate) async fn remove(Path(id): Path<String>) -> impl IntoResponse {
    let r = db::remove(TABLE, id.as_str());
    to_res(r)
}
