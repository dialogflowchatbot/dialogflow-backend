use super::context::Context;
use super::dto::{Request, Response};
use crate::flow::rt::node::RuntimeNode;
use crate::intent::detector;
use crate::result::{Error, Result};

pub(in crate::flow::rt) fn process(req: &mut Request) -> Result<Response> {
    // let now = std::time::Instant::now();
    let mut ctx = Context::get(&req.session_id);
    // println!("get ctx {:?}", now.elapsed());
    // let now = std::time::Instant::now();
    if ctx.no_node() {
        ctx.main_flow_id.push_str(&req.main_flow_id);
        ctx.add_node(&req.main_flow_id);
    }
    // println!("add_node {:?}", now.elapsed());
    // let now = std::time::Instant::now();
    if req.user_input_intent.is_none() {
        req.user_input_intent = detector::detect(&req.user_input)?;
        // println!("{:?}", req.user_input_intent);
    }
    if !req.import_variables.is_empty() {
        for v in req.import_variables.iter_mut() {
            let k = std::mem::take(&mut v.var_name);
            let v = crate::variable::dto::VariableValue::new(&v.var_val, &v.var_type);
            ctx.vars.insert(k, v);
        }
    }
    // println!("intent detect {:?}", now.elapsed());
    // let now = std::time::Instant::now();
    let r = exec(req, &mut ctx);
    // println!("exec {:?}", now.elapsed());
    // let now = std::time::Instant::now();
    ctx.save()?;
    // println!("ctx save {:?}", now.elapsed());
    r
}

pub(in crate::flow::rt) fn exec(req: &Request, ctx: &mut Context) -> Result<Response> {
    let mut response = Response::new();
    for _i in 0..100 {
        // let now = std::time::Instant::now();
        if let Some(n) = ctx.pop_node() {
            // println!("pop node {:?}", now.elapsed());
            let ret = n.exec(&req, ctx, &mut response);
            // println!("node exec {:?}", now.elapsed());
            if ret {
                return Ok(response);
            }
        } else {
            return Ok(response);
        }
    }
    Err(Error::ErrorWithMessage(String::from(
        "执行次数太多，请检查流程配置是否正确。",
    )))
}
