use std::vec::Vec;

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::Json;

use super::detector;
use super::dto::{Intent, IntentDetail, IntentFormData};
use crate::db;
use crate::result::{Error, Result};
use crate::web::server::to_res;

pub(crate) const INTENT_LIST_KEY: &str = "intents";
pub(crate) const TABLE: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new(INTENT_LIST_KEY);

pub(crate) fn init(is_en: bool) -> Result<()> {
    let mut intents: Vec<Intent> = Vec::with_capacity(2);

    // Positive
    let keywords = if is_en {
        vec![
            "sure",
            "ok",
            "okay",
            "no problem",
            "affim",
            "certainly",
            "of course",
            "definitely",
            "correct",
            "pleasant",
            "yes",
        ]
    } else {
        vec![
            "嗯", "恩", "可以", "是", "是的", "好", "好的", "对", "对的", "ok", "OK", "Ok", "知道",
            "明白", "行", "愿意", "方便", "正确",
        ]
    };
    let regexes: Vec<&str> = vec![];
    let intent_detail = IntentDetail {
        intent_idx: 0,
        keywords: keywords.into_iter().map(|s| String::from(s)).collect(),
        regexes: regexes.into_iter().map(|s| String::from(s)).collect(),
        phrases: vec![],
    };
    let mut intent = Intent::new(if is_en { "Positive" } else { "肯定" });
    intent.keyword_num = intent_detail.keywords.len();
    intent.regex_num = intent_detail.regexes.len();

    // let mut table = write_txn.open_table(TABLE)?;
    db::write(TABLE, intent.id.as_str(), &intent_detail)?;

    intents.push(intent);

    // Negative
    let keywords = if is_en {
        vec![
            "no",
            "not",
            "Reject",
            "can't",
            "can not",
            "cannot",
            "deny",
            "forbid",
            "forbidden",
            "stop",
            "gross",
            "impossible",
            "never",
            "rarely",
            "hardly",
            "none",
            "nothing",
            "incorrect",
            "awful",
            "unpleasant",
            "sick",
            "disappointed",
        ]
    } else {
        vec![
            "no",
            "不",
            "不是",
            "不要",
            "不用",
            "没用",
            "不好",
            "没",
            "没有",
            "不清楚",
            "不知道",
            "不明白",
            "不可以",
            "不行",
            "不愿意",
            "不考虑",
            "不方便",
            "不正确",
        ]
    };
    let regexes: Vec<&str> = vec![];
    let intent_detail = IntentDetail {
        intent_idx: 1,
        keywords: keywords.into_iter().map(|s| String::from(s)).collect(),
        regexes: regexes.into_iter().map(|s| String::from(s)).collect(),
        phrases: vec![],
    };
    let mut intent = Intent::new(if is_en { "Negative" } else { "否定" });
    intent.keyword_num = intent_detail.keywords.len();
    intent.regex_num = intent_detail.regexes.len();

    db::write(TABLE, intent.id.as_str(), &intent_detail)?;

    intents.push(intent);

    db::write(TABLE, INTENT_LIST_KEY, &intents)
}

pub(crate) async fn list() -> impl IntoResponse {
    let r: Result<Option<Vec<Intent>>> = db::query(TABLE, INTENT_LIST_KEY);
    to_res(r)
}

pub(crate) async fn add(Json(params): Json<IntentFormData>) -> impl IntoResponse {
    let r = add_intent(params.data.as_str());
    to_res(r)
}

fn add_intent(intent_name: &str) -> Result<()> {
    let d: Option<Vec<Intent>> = db::query(TABLE, INTENT_LIST_KEY)?;
    let mut intents = d.unwrap();
    let intent_idx = intents.len();
    let intent = Intent::new(intent_name);
    intents.push(intent.clone());
    db::write(TABLE, INTENT_LIST_KEY, &intents)?;

    let intent_detail = IntentDetail {
        intent_idx,
        keywords: vec![],
        regexes: vec![],
        phrases: vec![],
    };
    db::write(TABLE, intent.id.as_str(), &intent_detail)?;
    Ok(())
}

pub(crate) async fn remove(Json(params): Json<IntentFormData>) -> impl IntoResponse {
    let r = db::remove(TABLE, params.id.as_str());
    if let Ok(idx) = params.data.parse() {
        let mut intents: Vec<Intent> = db::query(TABLE, INTENT_LIST_KEY).unwrap().unwrap();
        intents.remove(idx);
        if let Err(e) = db::write(TABLE, INTENT_LIST_KEY, &intents) {
            log::error!("Update intents list failed: {:?}", &e);
        }
    }
    to_res(r)
}

pub(crate) async fn detail(Query(params): Query<IntentFormData>) -> impl IntoResponse {
    // let mut od: Option<IntentDetail> = None;
    // let r = db::process_data(dbg!(params.id.as_str()), |d: &mut IntentDetail| {
    //     od = Some(d);
    //     Ok(())
    // }).map(|_| od);
    // to_res(r)
    let r: Result<Option<IntentDetail>> = db::query(TABLE, params.id.as_str());
    to_res(r)
}

fn change_num<I: serde::Serialize, F: FnMut(&mut Vec<Intent>)>(
    key: &str,
    d: &mut I,
    mut f: F,
) -> Result<()> {
    let mut intents: Vec<Intent> = db::query(TABLE, INTENT_LIST_KEY).unwrap().unwrap();
    f(&mut intents);
    db::save_txn(vec![
        (TABLE, key, Box::new(d)),
        (TABLE, INTENT_LIST_KEY, Box::new(&intents)),
    ])
}

pub(crate) async fn add_keyword(Json(params): Json<IntentFormData>) -> impl IntoResponse {
    let key = params.id.as_str();
    let r: Result<Option<IntentDetail>> = db::query(TABLE, key);
    let r = r.and_then(|op| {
        if let Some(mut d) = op {
            d.keywords.push(String::from(params.data.as_str()));
            let idx = d.intent_idx;
            change_num(key, &mut d, |i: &mut Vec<Intent>| {
                i[idx].keyword_num = i[idx].keyword_num + 1
            })
        } else {
            Ok(())
        }
    });
    to_res(r)
}

pub(crate) async fn remove_keyword(Json(params): Json<IntentFormData>) -> impl IntoResponse {
    let r = params
        .data
        .parse::<usize>()
        .map_err(|e| {
            log::error!("{:?}", e);
            Error::ErrorWithMessage(String::from("Invalid parameter"))
        })
        .and_then(|idx| {
            let key = params.id.as_str();
            let result: Result<Option<IntentDetail>> = db::query(TABLE, key);
            result.and_then(|mut op| {
                if op.is_some() {
                    let mut d = op.as_mut().unwrap();
                    d.keywords.remove(idx);
                    let idx = d.intent_idx;
                    change_num(key, &mut d, |i: &mut Vec<Intent>| {
                        i[idx].keyword_num = i[idx].keyword_num - 1
                    })
                } else {
                    Ok(())
                }
            })
        });
    to_res(r)
}

pub(crate) async fn add_regex(Json(params): Json<IntentFormData>) -> impl IntoResponse {
    let key = params.id.as_str();
    let r: Result<Option<IntentDetail>> = db::query(TABLE, key);
    let r = r.and_then(|op| {
        if let Some(mut d) = op {
            let _ = regex::Regex::new(params.data.as_str())?;
            d.regexes.push(String::from(params.data.as_str()));
            let idx = d.intent_idx;
            change_num(key, &mut d, |i: &mut Vec<Intent>| {
                i[idx].regex_num = i[idx].regex_num + 1
            })
        } else {
            Ok(())
        }
    });
    to_res(r)
}

pub(crate) async fn remove_regex(Json(params): Json<IntentFormData>) -> impl IntoResponse {
    let r = params
        .data
        .parse::<usize>()
        .map_err(|e| {
            log::error!("{:?}", e);
            Error::ErrorWithMessage(String::from("Invalid parameter"))
        })
        .and_then(|idx| {
            let key = params.id.as_str();
            let result: Result<Option<IntentDetail>> = db::query(TABLE, key);
            result.and_then(|mut op| {
                if op.is_some() {
                    let mut d = op.as_mut().unwrap();
                    d.regexes.remove(idx);
                    let idx = d.intent_idx;
                    change_num(key, &mut d, |i: &mut Vec<Intent>| {
                        i[idx].regex_num = i[idx].regex_num - 1
                    })
                } else {
                    Ok(())
                }
            })
        });
    to_res(r)
}

pub(crate) async fn add_phrase(Json(params): Json<IntentFormData>) -> impl IntoResponse {
    let key = params.id.as_str();
    let r: Result<Option<IntentDetail>> = db::query(TABLE, key);
    if r.is_err() {
        return to_res(r.map(|_| ()));
    }
    let r = r.unwrap();
    if r.is_none() {
        return to_res(Err(Error::ErrorWithMessage(String::from(
            "Can NOT find intention detail",
        ))));
    }
    let mut d = r.unwrap();
    let r = detector::save_intent_embedding(key, &params.data).await;
    if r.is_err() {
        return to_res(r);
    }
    d.phrases.push(String::from(params.data.as_str()));
    let idx = d.intent_idx;
    let r = change_num(key, &mut d, |i: &mut Vec<Intent>| {
        i[idx].phrase_num = i[idx].phrase_num + 1
    });
    to_res(r)
}

pub(crate) async fn remove_phrase(Json(params): Json<IntentFormData>) -> impl IntoResponse {
    let r = params
        .data
        .parse::<usize>()
        .map_err(|e| {
            log::error!("{:?}", e);
            Error::ErrorWithMessage(String::from("Invalid parameter"))
        })
        .and_then(|idx| {
            let key = params.id.as_str();
            let result: Result<Option<IntentDetail>> = db::query(TABLE, key);
            result.and_then(|mut op| {
                if op.is_some() {
                    let mut d = op.as_mut().unwrap();
                    d.phrases.remove(idx);
                    let idx = d.intent_idx;
                    change_num(key, &mut d, |i: &mut Vec<Intent>| {
                        i[idx].phrase_num = i[idx].phrase_num - 1
                    })
                } else {
                    Ok(())
                }
            })
        });
    to_res(r)
}

pub(crate) async fn detect(Json(params): Json<IntentFormData>) -> impl IntoResponse {
    to_res(detector::detect(&params.data).await)
}
