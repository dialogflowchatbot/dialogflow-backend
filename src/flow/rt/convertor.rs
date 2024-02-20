use std::collections::{HashMap, HashSet};
use std::vec::Vec;

use super::condition::ConditionData;
use super::node::{
    CollectNode, ConditionNode, ExternalHttpCallNode, GotoAnotherNode, GotoMainFlowNode,
    RuntimeNnodeEnum, SendEmailNode, TerminateNode, TextNode,
};
use crate::db;
use crate::flow::demo;
use crate::flow::subflow::crud::TABLE;
use crate::flow::subflow::dto::{BranchType, CanvasCells, NextActionType, Node, SubFlowDetail};
use crate::result::{Error, Result};

pub(crate) fn convert_flow(mainflow_id: &str) -> Result<()> {
    let flows: Vec<SubFlowDetail> = if let Some(t) = demo::get_demo(mainflow_id) {
        serde_json::from_str(t)?
    } else {
        let r: Option<Vec<SubFlowDetail>> = db::query(TABLE, mainflow_id)?;
        if r.is_none() {
            return Err(Error::ErrorWithMessage(String::from("未找到流程数据")));
        }
        r.unwrap()
    };
    let mut idx = 0;
    for f in flows.iter() {
        // if !f.valid {
        //     return Err(Error::ErrorWithMessage(format!(
        //         "子流程：{} 校验不通过",
        //         f.name
        //     )));
        // }
        convert_subflow(mainflow_id, idx, f)?;
        idx = idx + 1;
    }
    Ok(())
}

fn validate_nodes(f: &SubFlowDetail, nodes: &Vec<&mut Node>) -> Result<()> {
    for node in nodes.iter() {
        node.is_valid(f)?;
    }
    Ok(())
}

fn check_first_node(
    mainflow_id: &str,
    flow_idx: usize,
    f: &SubFlowDetail,
    nodes: &mut Vec<&mut Node>,
) -> Result<()> {
    let mut ids: HashSet<String> = HashSet::with_capacity(16);
    for node in nodes.iter() {
        ids.insert(node.get_node_id());
    }
    for node in nodes.iter() {
        node.get_branch_target_ids().iter().for_each(|id| {
            ids.remove(id);
        });
    }
    if ids.len() == 1 {
        let mut id = String::with_capacity(36);
        for i in ids.drain() {
            id.push_str(i.as_str());
        }
        for node in nodes.iter_mut() {
            let first_node_id = if flow_idx == 0 { mainflow_id } else { &f.id };
            if node.get_node_id().eq(&id) {
                match node {
                    Node::DialogNode(ref mut n) => n.node_id = String::from(first_node_id),
                    Node::ConditionNode(n) => n.node_id = String::from(first_node_id),
                    Node::CollectNode(n) => n.node_id = String::from(first_node_id),
                    Node::GotoNode(n) => n.node_id = String::from(first_node_id),
                    Node::ExternalHttpNode(n) => n.node_id = String::from(first_node_id),
                    Node::SendEmailNode(n) => n.node_id = String::from(first_node_id),
                };
            }
        }
        Ok(())
    } else {
        Err(Error::ErrorWithMessage(format!(
            "Sub flow: {} can not find the start node.",
            f.name
        )))
    }
}

fn convert_subflow(mainflow_id: &str, flow_idx: usize, f: &SubFlowDetail) -> Result<()> {
    // println!("{}", &f.nodes);
    let mut cells: CanvasCells = serde_json::from_str(&f.canvas)?;
    let mut branches_link: HashMap<String, String> = HashMap::with_capacity(32);
    let mut node_cnt = 0usize;
    for n in cells.cells.iter() {
        if n.shape.eq("edge") {
            let source = n.extra.get("source");
            let port = source.as_ref().unwrap().get("port");
            let target = n.extra.get("target");
            let cell = target.as_ref().unwrap().get("cell");
            branches_link.insert(
                String::from(port.unwrap().as_str().unwrap()),
                String::from(cell.unwrap().as_str().unwrap()),
            );
        } else {
            if n.data.is_none() {
                return Err(Error::ErrorWithMessage(String::from(
                    "Node data information not found",
                )));
            }
            node_cnt = node_cnt + 1;
        }
    }
    // let mut inner_cells:&mut Vec<crate::flow::canvas::dto::CanvasCell> = cells.cells.as_mut();
    let mut nodes: Vec<&mut Node> = Vec::with_capacity(node_cnt);
    for n in cells.cells.iter_mut() {
        if let Some(node) = n.data.as_mut() {
            if let Some(branches) = node.get_branches() {
                for branch in branches.iter_mut() {
                    if branch.branch_id.is_empty() {
                        return Err(Error::ErrorWithMessage(String::from(
                            "Branch id information not found",
                        )));
                    }
                    let target_node_id = branches_link.remove(branch.branch_id.as_str());
                    if let Some(t) = target_node_id {
                        branch.target_node_id = t;
                    } else {
                        return Err(Error::ErrorWithMessage(String::from(
                            "Branch id information not found",
                        )));
                    }
                }
            }
            nodes.push(node);
        }
    }

    // let mut nodes: Vec<Node> = serde_json::from_str(&f.nodes)?;
    validate_nodes(f, &nodes)?;
    check_first_node(mainflow_id, flow_idx, f, &mut nodes)?;
    for node in nodes {
        convert_node(mainflow_id, node)?;
    }
    Ok(())
}

fn convert_node(main_flow_id: &str, node: &mut Node) -> Result<()> {
    let mut nodes: Vec<(String, rkyv::AlignedVec)> = Vec::with_capacity(32);
    match node {
        Node::DialogNode(n) => {
            let node = TextNode {
                text: n.dialog_text.clone(),
                ret: NextActionType::WaitUserResponse == n.next_step,
                next_node_id: n.branches[0].target_node_id.clone(),
            };
            // println!("Dialog {} {}", &n.node_id, &node.next_node_id);
            let r = RuntimeNnodeEnum::TextNode(node);
            let bytes = rkyv::to_bytes::<_, 256>(&r).unwrap();
            // let mut bytes = rkyv::to_bytes::<_, 256>(&node).unwrap();
            // bytes.push(RuntimeNodeTypeId::TextNode as u8);
            nodes.push((n.node_id.clone(), bytes));
        }
        Node::ConditionNode(n) => {
            // println!("Condition {}", &n.node_id);
            let mut cnt = 1u8;
            for b in n.branches.iter() {
                let node_id = if cnt == 1 {
                    n.node_id.clone()
                } else {
                    format!("{}-{}", &n.node_id, cnt)
                };
                cnt = cnt + 1;
                if BranchType::GotoAnotherNode == b.branch_type {
                    let node = GotoAnotherNode {
                        next_node_id: b.target_node_id.clone(),
                    };
                    let r = RuntimeNnodeEnum::GotoAnotherNode(node);
                    let bytes = rkyv::to_bytes::<_, 64>(&r).unwrap();
                    // bytes.push(RuntimeNodeTypeId::GotoAnotherNode as u8);
                    nodes.push((node_id, bytes));
                } else {
                    let mut conditions: Vec<Vec<ConditionData>> = Vec::with_capacity(10);
                    for and_condition in b.condition_group.iter() {
                        let mut and_conditions: Vec<ConditionData> = Vec::with_capacity(10);
                        for cond in and_condition.iter() {
                            let c = ConditionData {
                                condition_type: cond.condition_type,
                                compare_type: cond.compare_type,
                                ref_data: cond.ref_choice.clone(),
                                target_data: cond.target_value.clone(),
                                target_data_variant: cond.target_value_variant,
                            };
                            and_conditions.push(c);
                        }
                        conditions.push(and_conditions);
                    }
                    let node = ConditionNode {
                        next_node_id: format!("{}-{}", &n.node_id, cnt),
                        goto_node_id: b.target_node_id.clone(),
                        conditions: conditions,
                    };
                    let r = RuntimeNnodeEnum::ConditionNode(node);
                    let bytes = rkyv::to_bytes::<_, 256>(&r).unwrap();
                    // bytes.push(RuntimeNodeTypeId::ConditionNode as u8);
                    nodes.push((node_id, bytes));
                }
            }
        }
        Node::CollectNode(n) => {
            let mut successful_node_id = String::with_capacity(36);
            let mut failed_node_id = String::with_capacity(36);
            for b in n.branches.iter() {
                match b.branch_type {
                    BranchType::InfoCollectedSuccessfully => {
                        successful_node_id.push_str(b.target_node_id.as_str())
                    }
                    BranchType::GotoAnotherNode => {
                        failed_node_id.push_str(b.target_node_id.as_str())
                    }
                    _ => {
                        return Err(Error::ErrorWithMessage(String::from(
                            "Unknown collection branch type",
                        )))
                    }
                };
            }
            // println!("{} {}", &successful_node_id, &failed_node_id);
            let node = CollectNode {
                var_name: n.collect_save_var_name.clone(),
                collect_type: n.collect_type.clone(),
                successful_node_id: successful_node_id,
                failed_node_id: failed_node_id,
            };
            let r = RuntimeNnodeEnum::CollectNode(node);
            let bytes = rkyv::to_bytes::<_, 128>(&r).unwrap();
            // bytes.push(RuntimeNodeTypeId::CollectNode as u8);
            nodes.push((n.node_id.clone(), bytes));
        }
        Node::GotoNode(n) => {
            // println!("GotoNode {}", &n.node_id);
            match n.goto_type {
                NextActionType::Terminate => {
                    let node = TerminateNode {};
                    let r = RuntimeNnodeEnum::TerminateNode(node);
                    let bytes = rkyv::to_bytes::<_, 64>(&r).unwrap();
                    // bytes.push(RuntimeNodeTypeId::TerminateNode as u8);
                    nodes.push((n.node_id.clone(), bytes));
                }
                NextActionType::GotoMainFlow => {
                    let node = GotoMainFlowNode {
                        main_flow_id: n.goto_mainflow_id.clone(),
                        next_node_id: n.goto_subflow_id.clone(),
                    };
                    let r = RuntimeNnodeEnum::GotoMainFlowNode(node);
                    let bytes = rkyv::to_bytes::<_, 64>(&r).unwrap();
                    nodes.push((n.node_id.clone(), bytes));
                }
                NextActionType::GotoSubFlow => {
                    let node = GotoAnotherNode {
                        next_node_id: n.goto_subflow_id.clone(),
                    };
                    let r = RuntimeNnodeEnum::GotoAnotherNode(node);
                    let bytes = rkyv::to_bytes::<_, 64>(&r).unwrap();
                    // bytes.push(RuntimeNodeTypeId::GotoAnotherNode as u8);
                    nodes.push((n.node_id.clone(), bytes));
                }
                NextActionType::GotoExternalLink => {}
                _ => {}
            }
        }
        Node::ExternalHttpNode(n) => {
            let node = ExternalHttpCallNode {
                next_node_id: n.branches[0].target_node_id.clone(),
                http_api_id: n.http_api_id.clone(),
            };
            let r = RuntimeNnodeEnum::ExternalHttpCallNode(node);
            let bytes = rkyv::to_bytes::<_, 128>(&r).unwrap();
            // bytes.push(RuntimeNodeTypeId::CollectNode as u8);
            nodes.push((n.node_id.clone(), bytes));
        }
        Node::SendEmailNode(n) => {
            let (successful_node_id, goto_node_id) = {
                if n.async_send {
                    (
                        Some(std::mem::replace(
                            &mut n.branches[0].target_node_id,
                            String::new(),
                        )),
                        std::mem::replace(&mut n.branches[0].target_node_id, String::new()),
                    )
                } else {
                    (
                        None,
                        std::mem::replace(&mut n.branches[0].target_node_id, String::new()),
                    )
                }
            };
            let node = SendEmailNode {
                to_recipiants: std::mem::replace(&mut n.to_recipients, vec![]),
                cc_recipients: std::mem::replace(&mut n.cc_recipients, vec![]),
                bcc_recipients: std::mem::replace(&mut n.bcc_recipients, vec![]),
                subject: std::mem::replace(&mut n.subject, String::new()),
                content: std::mem::replace(&mut n.content, String::new()),
                async_send: n.async_send,
                successful_node_id: successful_node_id,
                goto_node_id: goto_node_id,
            };
            let r = RuntimeNnodeEnum::SendEmailNode(node);
            let bytes = rkyv::to_bytes::<_, 128>(&r).unwrap();
            // bytes.push(RuntimeNodeTypeId::CollectNode as u8);
            nodes.push((n.node_id.clone(), bytes));
        }
    };
    // let mut nodes: Vec<(&str, &[u8])> = Vec::with_capacity(box_nodes.len());
    // for n in box_nodes.iter() {
    // let json = serde_json::to_string(&n.1)?;
    // println!("saved {}", &n.0);
    // }

    super::crud::save_runtime_nodes(main_flow_id, nodes)
}

/*
#[derive(Archive, Deserialize, Serialize)]
#[archive(compare(PartialEq), check_bytes)]
struct MyNode {}

trait TestT {
    fn say(&self) -> String;
}

impl TestT for MyNode {
    fn say(&self) -> String {
        String::from("Into MyNode")
    }
}

pub fn t1() {
    let mut serializer = AllocSerializer::<512>::default();
    let nn: MyNode = MyNode {};
    serializer.serialize_value(&nn).unwrap();
    let mut bytes = serializer.into_serializer().into_inner();
    let b = bytes.as_slice();
    bytes.push(1u8);
    bytes.last();
    let archived = rkyv::check_archived_root::<MyNode>(&bytes[..]).unwrap();
    let deserialized: MyNode = archived.deserialize(&mut rkyv::Infallible).unwrap();
    t2(deserialized);
}

fn t2(n: impl TestT) {
    println!("{}", n.say());
}
*/
