use std::path::Path;

use axum::{
    extract::{Multipart, Query},
    response::IntoResponse,
};

use super::doc;
use crate::result::{Error, Result};
use crate::robot::dto::RobotQuery;
use crate::web::server::to_res;

pub(crate) async fn upload(Query(q): Query<RobotQuery>, multipart: Multipart) -> impl IntoResponse {
    if let Err(e) = do_uploading(&q.robot_id, multipart).await {
        return to_res(Err(e));
    }
    to_res(Ok(()))
}

async fn do_uploading(robot_id: &str, mut multipart: Multipart) -> Result<()> {
    let p = Path::new(".")
        .join("data")
        .join(robot_id)
        .join("kb")
        .join("docs")
        .join("upload");
    if !p.exists() {
        std::fs::create_dir_all(&p)?;
    }
    loop {
        let field = multipart.next_field().await?;
        if field.is_none() {
            return Ok(());
        }
        let field = field.unwrap();
        let Some(name) = field.name() else {
            return Err(Error::ErrorWithMessage(String::from("Name is missing.")));
        };
        let name = name.to_string();
        let Some(file_name) = field.file_name() else {
            return Err(Error::ErrorWithMessage(String::from(
                "File name is missing.",
            )));
        };
        let file_name = file_name.to_string();
        let Some(content_type) = field.content_type() else {
            return Err(Error::ErrorWithMessage(String::from(
                "Content type is missing.",
            )));
        };
        let content_type = content_type.to_string();
        let data = field.bytes().await?;

        log::info!(
            "Length of `{name}` (`{file_name}`: `{content_type}`) is {} bytes",
            data.len()
        );

        let text = doc::parse_docx(data.to_vec())?;
        log::info!("Extract text: {text}");
    }
}

pub(crate) async fn new_qa() -> impl IntoResponse {}
