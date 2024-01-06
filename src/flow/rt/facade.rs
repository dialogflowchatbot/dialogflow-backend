use axum::response::IntoResponse;
use axum::Json;

use super::dto::Request;
use super::executor;
use crate::web::server::to_res;

pub(crate) async fn answer(Json(mut req): Json<Request>) -> impl IntoResponse {
    let now = std::time::Instant::now();
    let r = executor::process(&mut req);
    // println!("exec used time:{:?}", now.elapsed());
    let res = to_res(r);
    println!("Total response used time:{:?}", now.elapsed());
    res
}
