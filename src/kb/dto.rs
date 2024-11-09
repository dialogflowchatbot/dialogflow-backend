use std::vec::Vec;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub(super) struct QuestionAnswer {
    pub(super) question: String,
    pub(super) similar_questions: Option<Vec<String>>,
    pub(super) answer: String,
}
