use std::collections::HashMap;

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::ai::completion;

#[derive(Deserialize, Serialize)]
pub(crate) struct Request {
    pub(crate) robot_id: String,
    pub(crate) prompt: String,
}

pub(crate) async fn gen_text(Json(q): Json<Request>) -> impl IntoResponse {
    if q.robot_id.is_empty() || q.prompt.is_empty() {}
    completion::completion(&q.robot_id, "system_hint", &q.prompt).await;
    ""
}
