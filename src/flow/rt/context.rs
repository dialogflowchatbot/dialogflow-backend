use std::collections::{HashMap, LinkedList};
// use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec::Vec;

// use erased_serde::{Deserialize, Serialize};
use serde::{Deserialize, Serialize};
use tokio::time::{interval, Duration};

use super::node::RuntimeNnodeEnum;
use crate::db;
use crate::result::Result;
use crate::variable::dto::VariableValue;

const TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("contexts");
pub(crate) const CONTEXT_KEY: &str = "contexts";

#[derive(Deserialize, Serialize)]
pub(crate) struct ContextStatus {
    session_id: String,
    create_time: u64,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Context {
    pub(in crate::flow::rt) main_flow_id: String,
    session_id: String,
    pub(in crate::flow::rt) nodes: LinkedList<String>,
    pub(crate) vars: HashMap<String, VariableValue>,
    #[serde(skip)]
    pub(crate) none_persistent_vars: HashMap<String, VariableValue>,
    #[serde(skip)]
    pub(crate) none_persistent_data: HashMap<String, String>,
}

impl Context {
    pub(crate) fn get(session_id: &str) -> Self {
        if let Ok(o) = db::query(TABLE, session_id) {
            if let Some(ctx) = o {
                return ctx;
            }
        }
        let r: Result<Option<Vec<ContextStatus>>> = db::query(TABLE, CONTEXT_KEY);
        if let Ok(op) = r {
            if let Some(mut d) = op {
                let ctx = ContextStatus {
                    session_id: String::from(session_id),
                    create_time: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
                d.push(ctx);
                if let Err(e) = db::write(TABLE, CONTEXT_KEY, &d) {
                    eprint!("{:?}", e);
                }
            }
        }
        let ctx = Self {
            main_flow_id: String::with_capacity(64),
            session_id: String::from(session_id),
            nodes: LinkedList::new(),
            vars: HashMap::with_capacity(16),
            none_persistent_vars: HashMap::with_capacity(16),
            none_persistent_data: HashMap::with_capacity(16),
        };
        ctx
    }

    pub(crate) fn save(&self) -> Result<()> {
        db::write(TABLE, self.session_id.as_str(), self)
    }

    // pub(crate) fn clear(&mut self) -> Result<()> {
    //     self.nodes.clear();
    //     self.vars.clear();
    //     self.save()
    // }

    pub(in crate::flow::rt) fn no_node(&self) -> bool {
        self.nodes.is_empty()
    }

    pub(in crate::flow::rt) fn add_node(&mut self, node_id: &str) {
        // print!("add_node {} ", node_id);
        self.nodes.push_front(String::from(node_id));
        // let now = std::time::Instant::now();
        // if let Ok(r) = db::get_runtime_node(node_id) {
        //     if let Some(n) = r {
        //         self.nodes.push_front(n);
        //         // println!("added");
        //     }
        // }
        // println!("add_node used time:{:?}", now.elapsed());
    }

    pub(in crate::flow::rt) fn pop_node(&mut self) -> Option<RuntimeNnodeEnum> {
        // println!("nodes len {}", self.nodes.len());
        if let Some(node_id) = self.nodes.pop_front() {
            // println!("main_flow_id {} node_id {}", &self.main_flow_id, &node_id);
            if let Ok(r) = super::crud::get_runtime_node(&self.main_flow_id, &node_id) {
                return r;
            }
        }
        None
    }
}

pub(crate) fn init() -> Result<()> {
    db::write(TABLE, CONTEXT_KEY, &String::from("[]"))
}

pub async fn clean_expired_session(
    mut recv: tokio::sync::oneshot::Receiver<()>,
    max_session_duration_min: u16,
) {
    let max_sess_dur_sec = (max_session_duration_min * 60) as u64;
    let mut interval = interval(Duration::from_secs(max_sess_dur_sec));
    loop {
        // https://docs.rs/tokio/latest/tokio/sync/oneshot/index.html
        // https://users.rust-lang.org/t/how-can-i-terminate-a-tokio-task-even-if-its-not-finished/40641
        tokio::select! {
          _ = interval.tick() => {
          }
          _ = &mut recv => {
            break;
          }
        }
        // sleep(Duration::from_millis(1800000)).await;
        log::info!("Cleaning expired sessions");
        let r: Result<Option<Vec<ContextStatus>>> = db::query(TABLE, CONTEXT_KEY);
        if let Ok(op) = r {
            if let Some(mut d) = op {
                match SystemTime::now().duration_since(UNIX_EPOCH) {
                    Ok(dura) => {
                        let now = dura.as_secs();
                        let mut i = 0;
                        while i < d.len() {
                            // println!("{} {}", now, d[i].create_time);
                            if now - d[i].create_time > max_sess_dur_sec {
                                let val = d.remove(i);
                                if let Err(e) = db::remove(TABLE, val.session_id.as_str()) {
                                    log::error!(
                                        "Removing expired session {} failed {:?}",
                                        val.session_id,
                                        e
                                    );
                                }
                            } else {
                                i += 1;
                            }
                        }
                        if let Err(e) = db::write(TABLE, CONTEXT_KEY, &d) {
                            log::error!("{:?}", e);
                        }
                    }
                    Err(e) => log::error!("{:?}", e),
                }
            }
        }
    }
}
