use axum::extract::{Query, Request};
use axum::http::{header::HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use once_cell::sync::Lazy;
use redb::TableDefinition;
use tokio::sync::Mutex;

use super::dto::{SubFlowDetail, SubFlowFormData};
use crate::db;
use crate::flow::demo;
use crate::result::{Error, Result};
use crate::web::server::to_res;

pub(crate) const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("subflows");
// pub(crate) const SUB_FLOW_LIST_KEY: &str = "subflows";
static LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

// pub(crate) fn init(is_en: bool, mainflow_id: &str) -> Result<()> {
//     let flow = vec![SubFlowDetail::new(if is_en {
//         "First sub-flow"
//     } else {
//         "第一个子流程"
//     })];
//     db::write(TABLE, mainflow_id, &flow)
// }

pub(crate) async fn list(headers: HeaderMap, Query(q): Query<SubFlowFormData>) -> Response {
    let client_language = headers
        .get("Accept-Language")
        .map_or_else(|| "", |v| v.to_str().unwrap_or(""));
    let template = demo::get_demo(client_language, &q.main_flow_id);
    if template.is_some() {
        return (StatusCode::OK, template.unwrap()).into_response();
    }
    to_res::<Option<Vec<SubFlowDetail>>>(db::query(TABLE, q.main_flow_id.as_str())).into_response()
    // let r = db::process_data(FLOW_LIST_KEY, |mut flows: Vec<FlowDetail>| {
    //     flows.iter_mut().for_each(|f| f.nodes.clear());
    //     Ok(flows)
    // });
    // to_res(r)
}

pub(crate) async fn simple_list(Query(q): Query<SubFlowFormData>) -> Response {
    let r: Result<Option<Vec<SubFlowDetail>>> = db::query(TABLE, q.main_flow_id.as_str());
    if let Ok(op) = r {
        if let Some(mut d) = op {
            for f in d.iter_mut() {
                f.canvas.clear();
            }
            return to_res::<Vec<SubFlowDetail>>(Ok(d)).into_response();
        }
    }
    "[]".into_response()
}

pub(crate) async fn new_subflow(
    mainflow_id: &str,
    subflow_name: &str,
) -> Result<Vec<SubFlowDetail>> {
    let _ = LOCK.lock().await;
    db::query(TABLE, mainflow_id)
        .map(|op: Option<Vec<SubFlowDetail>>| {
            let mut subflow = SubFlowDetail::new(subflow_name);
            let subflows = {
                if let Some(mut flows) = op {
                    flows.push(subflow);
                    flows
                } else {
                    subflow.id.clear();
                    subflow.id.push_str(mainflow_id);
                    vec![subflow]
                }
            };
            subflows
        })
        .and_then(|subflows| {
            db::write(TABLE, mainflow_id, &subflows)?;
            Ok(subflows)
        })
}

pub(crate) async fn new(Query(form): Query<SubFlowFormData>) -> impl IntoResponse {
    to_res(new_subflow(&form.main_flow_id, &form.data).await)
}

pub(crate) async fn save(
    Query(form): Query<SubFlowFormData>,
    Json(data): Json<SubFlowDetail>,
) -> impl IntoResponse {
    let r: Result<Vec<SubFlowDetail>> = form
        .data
        .parse::<usize>()
        .map_err(|e| Error::ErrorWithMessage(format!("{:?}", e)))
        .and_then(|idx| {
            let op: Option<Vec<SubFlowDetail>> = db::query(TABLE, form.main_flow_id.as_str())?;
            if let Some(mut flows) = op {
                if let Some(flow) = flows.get_mut(idx) {
                    flow.canvas = data.canvas.clone();
                    db::write(TABLE, &form.main_flow_id, &flows)?;
                }
                Ok(flows)
            } else {
                Ok(vec![])
            }
        });
    to_res(r)
}

pub(crate) async fn delete(Query(form): Query<SubFlowFormData>) -> impl IntoResponse {
    let r = form
        .data
        .parse::<usize>()
        .map_err(|e| Error::ErrorWithMessage(format!("{:?}", e)))
        .and_then(|idx| {
            let result: Result<Option<Vec<SubFlowDetail>>> =
                db::query(TABLE, form.main_flow_id.as_str());
            if let Ok(op) = result {
                if let Some(mut flows) = op {
                    if idx < flows.len() {
                        flows.remove(idx);
                        db::write(TABLE, &form.main_flow_id, &flows)?;
                    }
                }
            }
            Ok(())
        });
    to_res(r)
}

pub(crate) async fn release(
    headers: HeaderMap,
    Query(data): Query<SubFlowFormData>,
) -> impl IntoResponse {
    // let now = std::time::Instant::now();
    let client_language = headers
        .get("Accept-Language")
        .map_or_else(|| "", |v| v.to_str().unwrap_or(""));
    let r = crate::flow::rt::convertor::convert_flow(client_language, &data.main_flow_id);
    // println!("release used time:{:?}", now.elapsed());
    to_res(r)
}

pub(crate) async fn output(Query(data): Query<SubFlowFormData>) -> impl IntoResponse {
    let flows: Option<Vec<SubFlowDetail>> = db::query(TABLE, data.main_flow_id.as_str()).unwrap();
    serde_json::to_string(&flows).unwrap()
}
