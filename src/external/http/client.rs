use std::collections::HashMap;
use std::time::Duration;
use std::vec::Vec;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::RequestBuilder;

use super::dto::{HttpReqInfo, Method, PostContentType, Protocol, ResponseData, ValueSource};
use crate::variable::dto::VariableValue;

pub(crate) async fn req_async(
    info: HttpReqInfo,
    vars: HashMap<String, VariableValue>,
    ignore_response: bool,
) -> reqwest::Result<ResponseData> {
    req(info, &vars, ignore_response).await
}

pub(crate) async fn req(
    info: HttpReqInfo,
    vars: &HashMap<String, VariableValue>,
    ignore_response: bool,
) -> reqwest::Result<ResponseData> {
    let req = build_req(&info, vars)?;
    let res = req.send().await?;
    // println!("http status code {}", res.status().as_str());
    if ignore_response || res.status().as_u16() != 200 {
        return Ok(ResponseData::None);
    }
    let content_type = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .map_or("", |h| h.to_str().unwrap());
    let data = if content_type.find("text/").is_some()
        || content_type.find("/json").is_some()
        || content_type.find("/xml").is_some()
    {
        let s = res.text().await?;
        // println!("{}", s);
        ResponseData::Str(s)
    } else {
        ResponseData::Bin(res.bytes().await?.to_vec())
    };
    Ok(data)
}

fn build_req(
    info: &HttpReqInfo,
    vars: &HashMap<String, VariableValue>,
) -> reqwest::Result<RequestBuilder> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(1000))
        .timeout(Duration::from_millis(info.timedout_milliseconds))
        .build()?;
    let mut url = String::with_capacity(512);
    match info.protocol {
        Protocol::HTTP => url.push_str("http"),
        Protocol::HTTPS => url.push_str("https"),
    }
    url.push_str("://");
    url.push_str(&info.address);
    let req = match info.method {
        Method::GET => client.get(&url),
        Method::POST => {
            let r = client.post(&url);
            if !info.request_body.is_empty() {
                r.body(info.request_body.clone())
            } else {
                r
            }
        }
    };
    let req = if info.headers.is_empty() {
        req
    } else {
        let mut headers: HeaderMap<HeaderValue> = HeaderMap::with_capacity(info.headers.len());
        for p in info.headers.iter() {
            match p.value_source {
                ValueSource::Val => headers.insert(
                    HeaderName::from_bytes(p.name.as_bytes()).unwrap(),
                    p.value.parse().unwrap(),
                ),
                ValueSource::Var => headers.insert(
                    HeaderName::from_bytes(p.name.as_bytes()).unwrap(),
                    vars.get(&p.value)
                        .map_or(String::new(), |v| v.val_to_string())
                        .parse()
                        .unwrap(),
                ),
            };
        }
        req.headers(headers)
    };
    let req = if info.query_params.is_empty() {
        req
    } else {
        let mut queries: Vec<(&str, String)> = Vec::with_capacity(info.query_params.len());
        for p in info.query_params.iter() {
            match p.value_source {
                ValueSource::Val => queries.push((&p.name, p.value.clone())),
                ValueSource::Var => queries.push((
                    &p.name,
                    vars.get(&p.value)
                        .map_or(String::new(), |v| v.val_to_string()),
                )),
            }
        }
        req.query(&queries)
    };
    let req = if info.post_content_type == PostContentType::JSON {
        req.header("Content-Type", "application/json")
    } else {
        req
    };
    let req = if info.user_agent.is_empty() {
        req
    } else {
        req.header("Content-Type", &info.user_agent)
    };
    Ok(req.timeout(Duration::from_millis(info.timedout_milliseconds)))
}
