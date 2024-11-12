use arrow_array::types::Float32Type;
use arrow_array::{FixedSizeListArray, Int32Array, RecordBatch, RecordBatchIterator};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;

use lancedb::arrow::IntoArrow;
use lancedb::connection::Connection;
use lancedb::index::Index;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{connect, Error, Result, Table as LanceDbTable};

async fn add() -> Result<()> {
    let db = connect("data/embeddings").execute().await?;
    let table = db.open_table("my_table").execute().await?;
    Ok(())
}
