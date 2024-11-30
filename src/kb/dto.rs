use std::vec::Vec;

use serde::{Deserialize, Serialize};

// #[derive(Deserialize, Serialize)]
// pub(crate) struct QuestionAnswerData {
//     pub(super) id: Option<String>,
//     #[serde(rename = "qaData")]
//     pub(super) qa_data: QuestionAnswerPair,
// }

#[derive(Deserialize, Serialize)]
pub(crate) struct QuestionAnswerPair {
    pub(super) id: Option<String>,
    pub(super) question: QuestionData,
    #[serde(rename = "similarQuestions")]
    pub(super) similar_questions: Vec<QuestionData>,
    pub(crate) answer: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct QuestionData {
    pub(super) question: String,
    pub(super) vec_row_id: Option<i64>,
}
