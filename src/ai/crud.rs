use core::time::Duration;
use std::convert::Infallible;
use std::result::Result;

use axum::response::sse::{Event, Sse};
use axum::Json;
// use crossbeam_channel::bounded;
use futures::future::Either;
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt as _;

use crate::ai::completion;

#[derive(Deserialize, Serialize)]
pub(crate) struct Request {
    pub(crate) robot_id: String,
    pub(crate) prompt: String,
}

pub(crate) async fn gen_text(
    Json(q): Json<Request>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = if q.robot_id.is_empty() || q.prompt.is_empty() {
        Either::Left(stream::once(futures::future::ready(Ok(
            Event::default().data("Invalid robot_id or prompt")
        ))))
    } else {
        // let (sender, receiver) = bounded::<String>(1);
        let (sender, receiver) = mpsc::channel::<String>(1);
        let stream = ReceiverStream::new(receiver);
        if let Err(e) = completion::completion(&q.robot_id, &q.prompt, sender).await {
            log::error!("{:?}", &e);
        }
        Either::Right(stream.map(|s| Ok(Event::default().data(s))))
    };
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keep-alive-text"),
    )
}
