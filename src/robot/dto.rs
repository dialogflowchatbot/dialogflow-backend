use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub(crate) enum RobotType {
    Inbound,
    Outbound,
    Text,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct RobotData {
    pub(crate) robot_id: String,
    pub(crate) robot_name: String,
    pub(crate) robot_type: RobotType,
}
