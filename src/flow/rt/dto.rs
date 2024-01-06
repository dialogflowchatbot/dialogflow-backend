use std::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::{flow::subflow::dto::NextActionType, variable::dto::SimpleVariable};

#[derive(Deserialize, PartialEq, Eq)]
pub(crate) enum UserInputResult {
    Successful,
    Timeout,
}

#[derive(Deserialize)]
pub(crate) struct Request {
    #[serde(rename = "mainFlowId")]
    pub(crate) main_flow_id: String,
    #[serde(rename = "sessionId")]
    pub(crate) session_id: String,
    #[serde(rename = "userInputResult")]
    pub(crate) user_input_result: UserInputResult,
    #[serde(rename = "userInput")]
    pub(crate) user_input: String,
    #[serde(rename = "importVariables")]
    pub(crate) import_variables: Vec<SimpleVariable>,
    #[serde(rename = "userInputIntent")]
    pub(crate) user_input_intent: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct CollectData {
    pub(crate) var_name: String,
    pub(crate) value: String,
}

#[derive(Serialize)]
pub(crate) struct Response {
    pub(crate) answers: Vec<String>,
    #[serde(rename = "collectData")]
    pub(crate) collect_data: Vec<CollectData>,
    #[serde(rename = "nextAction")]
    pub(crate) next_action: NextActionType,
    #[serde(rename = "extraData")]
    pub(crate) extra_data: ExtraData,
}

impl Response {
    pub(crate) fn new() -> Self {
        Self {
            answers: Vec::with_capacity(5),
            collect_data: Vec::with_capacity(10),
            next_action: NextActionType::None,
            extra_data: ExtraData {
                external_link: String::new(),
            },
        }
    }
}

#[derive(Serialize)]
pub(crate) struct ExtraData {
    #[serde(rename = "externalLink")]
    pub(crate) external_link: String,
}
