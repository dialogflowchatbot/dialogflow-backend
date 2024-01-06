use regex::Regex;

use super::dto::{Intent, IntentDetail};
use crate::db;
use crate::result::Result;

pub(crate) fn detect(s: &str) -> Result<Option<String>> {
    // let now = std::time::Instant::now();
    let op: Option<Vec<Intent>> = db::query(super::crud::TABLE, super::crud::INTENT_LIST_KEY)?;
    // println!("inner intent detect {:?}", now.elapsed());
    if let Some(r) = op {
        for i in r.iter() {
            let r: Option<IntentDetail> = db::query(super::crud::TABLE, i.id.as_str())?;
            if let Some(detail) = r {
                for k in detail.keywords.iter() {
                    if k.eq(s) {
                        // println!("{} {} {}", s, k, &i.name);
                        return Ok(Some(i.name.clone()));
                    }
                }
                for r in detail.regexes.iter() {
                    let re = Regex::new(r)?;
                    if re.is_match(s) {
                        return Ok(Some(i.name.clone()));
                    }
                }
            }
        }
    }
    Ok(None)
}
