use std::vec::Vec;

use axum::extract::Query;
use axum::{response::IntoResponse, Json};
use redb::TableDefinition;

use super::dto::{RobotData, RobotQuery, RobotType};
use crate::db_executor;
use crate::external::http::crud as http;
use crate::flow::mainflow::crud as mainflow;
use crate::intent::crud as intent;
use crate::man::settings;
use crate::result::{Error, Result};
use crate::variable::crud as variable;
use crate::web::server;
use crate::{db, web::server::to_res};

const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("robots");

pub(crate) fn init(is_en: bool) -> Result<()> {
    db::init_table(TABLE)?;
    let name = if is_en {
        "My first robot"
    } else {
        "我的第一个机器人"
    };
    let d = RobotData {
        robot_id: scru128::new_string(),
        robot_name: String::from(name),
        robot_type: RobotType::TextBot,
    };
    new(&d, is_en)
}

pub(crate) async fn save(
    headers: axum::http::HeaderMap,
    Json(mut d): Json<RobotData>,
) -> impl IntoResponse {
    if d.robot_id.is_empty() {
        let is_en = server::is_en(&headers);
        d.robot_id = scru128::new_string();
        if let Err(e) = new(&d, is_en) {
            return to_res(Err(Error::ErrorWithMessage(format!(
                "Failed to create robot, error detail was: {:?}",
                &e
            ))));
        }
    }
    let r = persist(&d);
    to_res(r)
}

fn new(d: &RobotData, is_en: bool) -> Result<()> {
    persist(d)?;
    // 机器人意图
    settings::init(&d.robot_id)?;
    // 意图
    intent::init(&d.robot_id, is_en)?;
    // 变量
    variable::init(&d.robot_id, is_en)?;
    // 主流程
    mainflow::init(&d.robot_id)?;
    // Http 接口
    http::init(&d.robot_id)?;
    Ok(())
}

fn persist(d: &RobotData) -> Result<()> {
    db::write(TABLE, d.robot_id.as_str(), &d)?;
    Ok(())
}

pub(crate) async fn list() -> impl IntoResponse {
    to_res::<Vec<RobotData>>(db::get_all(TABLE))
}

pub(crate) async fn detail(Query(q): Query<RobotQuery>) -> impl IntoResponse {
    to_res::<Option<RobotData>>(db::query(TABLE, q.robot_id.as_str()))
}

pub(crate) async fn delete(Query(q): Query<RobotQuery>) -> impl IntoResponse {
    to_res(purge(&q.robot_id))
}

fn purge(robot_id: &str) -> Result<()> {
    std::fs::remove_dir(format!(
        "{}{}",
        crate::intent::detector::SAVING_PATH_ROOT,
        robot_id
    ))?;
    db::remove(crate::man::settings::TABLE, robot_id)?;
    db_executor!(
        db::delete_table,
        robot_id,
        crate::external::http::crud::TABLE_SUFFIX,
    )?;
    db_executor!(
        db::delete_table,
        robot_id,
        crate::variable::crud::TABLE_SUFFIX,
    )?;
    db_executor!(
        db::delete_table,
        robot_id,
        crate::intent::crud::TABLE_SUFFIX,
    )?;
    db_executor!(
        db::delete_table,
        robot_id,
        crate::flow::subflow::crud::TABLE_SUFFIX,
    )?;
    let r: Vec<crate::flow::mainflow::dto::MainFlowDetail> = db_executor!(
        db::get_all,
        robot_id,
        crate::flow::mainflow::crud::TABLE_SUFFIX,
    )?;
    for v in r.iter() {
        crate::flow::rt::crud::remove_runtime_nodes(&v.id)?;
    }
    db_executor!(
        db::delete_table,
        robot_id,
        crate::flow::mainflow::crud::TABLE_SUFFIX,
    )?;
    Ok(())
}
