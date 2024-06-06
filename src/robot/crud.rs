use std::borrow::Borrow;
use std::vec::Vec;

use axum::extract::Query;
use axum::{response::IntoResponse, Json};
use redb::TableDefinition;

use super::dto::{RobotData, RobotQuery, RobotType};
use crate::db_executor;
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
    let r = persist("", name, RobotType::Text)?;
    Ok(r.robot_id)
}

pub(crate) async fn save(Json(d): Json<RobotData>) -> impl IntoResponse {
    let r = persist(&d.robot_id, &d.robot_name, d.robot_type);
    to_res(r)
}

fn persist(id: &str, name: &str, r_type: RobotType) -> Result<RobotData> {
    let r = RobotData {
        robot_id: if id.is_empty() {
            scru128::new_string()
        } else {
            String::from(id)
        },
        robot_name: String::from(name),
        robot_type: r_type,
    };
    db::write(TABLE, r.robot_id.as_str(), &r)?;
    Ok(r)
}

pub(crate) async fn list(Query(q): Query<RobotQuery>) -> impl IntoResponse {
    to_res::<Vec<RobotData>>(db_executor!(db::get_all, &q.robot_id, "",))
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
