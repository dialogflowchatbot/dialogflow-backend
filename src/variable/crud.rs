// use std::borrow::BorrowMut;
use std::vec::Vec;

use axum::response::IntoResponse;
use axum::Json;

use super::dto::Variable;
use super::dto::{VariableObtainValueExpressionType, VariableType, VariableValueSource};
use crate::db;
use crate::flow::rt::context::Context;
use crate::flow::rt::dto::Request;
use crate::result::Result;
use crate::web::server::to_res;

const TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("variables");
// pub(crate) const VARIABLE_LIST_KEY: &str = "variables";

pub(crate) fn init(robot_id: &str, is_en: bool) -> Result<()> {
    let v = Variable {
        var_name: String::from(if is_en {
            "CollectionVar"
        } else {
            "采集变量"
        }),
        var_type: VariableType::Str,
        var_val_source: VariableValueSource::Collect,
        var_constant_value: String::new(),
        var_associate_data: String::new(),
        obtain_value_expression_type: VariableObtainValueExpressionType::None,
        obtain_value_expression: String::new(),
        cach_enabled: true,
    };
    let table_name = format!("{}vars", robot_id);
    let table: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new(&table_name);
    db::write(table, &v.var_name, &v)
}

pub(crate) async fn list() -> impl IntoResponse {
    to_res::<Vec<Variable>>(db::get_all(TABLE))
}

pub(crate) async fn add(Json(v): Json<Variable>) -> impl IntoResponse {
    /*
    let r: Result<Option<Vec<Variable>>> = db::query(TABLE, VARIABLE_LIST_KEY);
    let r = r.and_then(|op| {
        let mut new_record = true;
        if let Some(mut d) = op {
            d.retain_mut(|x| {
                if x.var_name.eq(&v.var_name) {
                    x.var_type = v.var_type.clone();
                    x.var_val_source = v.var_val_source.clone();
                    new_record = false;
                }
                true
            });
            if new_record {
                d.push(v.clone());
            }
            db::write(TABLE, VARIABLE_LIST_KEY, &d)
        } else {
            let d = vec![v];
            db::write(TABLE, VARIABLE_LIST_KEY, &d)
        }
    });
    to_res(r)
    */
    to_res(db::write(TABLE, &v.var_name, &v))
}

// fn t1<R, F: FnMut(String) -> R>(s: String, mut f: F) -> R {
//     f(s)
// }

// fn t2() -> Result<()> {
//     let r = t1(String::new(), |s| Ok(()));
//     r
// }

pub(crate) async fn delete(Json(v): Json<Variable>) -> impl IntoResponse {
    /*
    let r = v
        .var_name
        .parse::<usize>()
        .map_err(|e| Error::ErrorWithMessage(format!("{:?}", e)))
        .and_then(|idx| {
            let mut op: Option<Vec<Variable>> = db::query(TABLE, VARIABLE_LIST_KEY)?;
            if op.is_some() {
                let d = op.as_mut().unwrap();
                d.remove(idx);
                db::write(TABLE, VARIABLE_LIST_KEY, &d)?;
            }
            Ok(())
        });
    to_res(r)
    */
    to_res(db::remove(TABLE, v.var_name.as_str()))
}

pub(crate) fn get(name: &str) -> Result<Option<Variable>> {
    /*
    db::query(TABLE, VARIABLE_LIST_KEY).and_then(|op: Option<Vec<Variable>>| {
        if let Some(d) = op {
            for v in d {
                if v.var_name.eq(name) {
                    return Ok(Some(v.clone()));
                }
            }
        }
        return Ok(None);
    })
    */
    db::query(TABLE, name)
}

pub(crate) fn get_value(name: &str, req: &Request, ctx: &mut Context) -> String {
    if let Ok(r) = get(name) {
        if let Some(v) = r {
            if let Some(val) = v.get_value(req, ctx) {
                return val.val_to_string();
            }
        }
    }
    String::new()
}
