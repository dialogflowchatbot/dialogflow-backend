use std::vec::Vec;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub(super) struct QuestionAnswersPair {
    pub(super) question: QuestionData,
    pub(super) similar_questions: Option<Vec<QuestionData>>,
    pub(super) answer: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct QuestionData {
    pub(super) question: String,
    pub(super) vec_row_id: Option<i64>,
}
