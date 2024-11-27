use std::vec::Vec;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub(crate) struct QuestionAnswerData {
    pub(super) id: String,
    #[serde(rename = "qaData")]
    pub(super) qa_data: QuestionAnswerPair,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct QuestionAnswerPair {
    pub(super) question: QuestionData,
    #[serde(rename = "similarQuestions")]
    pub(super) similar_questions: Option<Vec<QuestionData>>,
    pub(super) answer: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct QuestionData {
    pub(super) question: String,
    pub(super) vec_row_id: Option<i64>,
}
