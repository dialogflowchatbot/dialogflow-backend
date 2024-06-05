use axum::{response::IntoResponse, Json};
use redb::TableDefinition;

use super::dto::{RobotData, RobotType};
use crate::result::Result;
use crate::{db, web::server::to_res};

const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("robots");

pub(crate) fn init(is_en: bool) -> Result<String> {
    db::init_table(TABLE)?;
    let name = if is_en {
        "My first robot"
    } else {
        "我的第一个机器人"
    };
    let r = persist(name, RobotType::Text)?;
    Ok(r.robot_id)
}

pub(crate) async fn save(Json(d): Json<RobotData>) -> impl IntoResponse {
    let r = persist(&d.robot_name, d.robot_type);
    to_res(r)
}

pub(crate) fn persist(name: &str, r_type: RobotType) -> Result<RobotData> {
    let r = RobotData {
        robot_id: scru128::new_string(),
        robot_name: String::from(name),
        robot_type: r_type,
    };
    db::write(TABLE, r.robot_id.as_str(), &r)?;
    Ok(r)
}

pub(crate) async fn list() -> impl IntoResponse {
    ""
}
