use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub(crate) struct RobotQuery {
    #[serde(rename = "robotId")]
    pub(crate) robot_id: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) enum RobotType {
    Inbound,
    Outbound,
    Text,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct RobotData {
    #[serde(rename = "robotId")]
    pub(crate) robot_id: String,
    #[serde(rename = "robotName")]
    pub(crate) robot_name: String,
    #[serde(rename = "robotType")]
    pub(crate) robot_type: RobotType,
}
