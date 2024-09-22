use axum::{extract::Multipart, response::IntoResponse};

use crate::result::Error;
use crate::web::server::to_res;

pub(crate) async fn upload(mut multipart: Multipart) -> impl IntoResponse {
    loop {
        let r = multipart.next_field().await;
        if r.is_err() {
            let m = format!("Upload failed, err: {:?}.", r.unwrap_err());
            return to_res(Err(Error::ErrorWithMessage(m)));
        }
        let field = r.unwrap();
        if field.is_none() {
            return to_res(Ok("Upload successfully."));
        }
        let field = field.unwrap();
        let name = field.name().unwrap().to_string();
        let file_name = field.file_name().unwrap().to_string();
        let content_type = field.content_type().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        println!(
            "Length of `{name}` (`{file_name}`: `{content_type}`) is {} bytes",
            data.len()
        );
    }
}