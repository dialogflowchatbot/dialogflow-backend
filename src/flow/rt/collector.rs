use serde::{Deserialize, Serialize};
use {once_cell::sync::Lazy, regex::Regex};

static NUMBER_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"[1-9]([\d]+)?(.[\d]+)?").unwrap());

#[derive(Clone, Deserialize, Serialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[archive(compare(PartialEq), check_bytes)]
pub(crate) enum CollectType {
    UserInput,
    Number,
    IdCard,
    CustomizeRegex(String),
}

pub(crate) fn collect<'a>(s: &'a str, collect_type: &CollectType) -> Option<&'a str> {
    match collect_type {
        CollectType::UserInput => Some(s),
        CollectType::Number => {
            if let Some(cap) = NUMBER_REGEX.captures(s) {
                if let Some(m) = cap.get(0) {
                    return Some(&s[m.start()..m.end()]);
                }
            }
            None
        }
        CollectType::IdCard => todo!(),
        CollectType::CustomizeRegex(regex) => {
            if let Ok(re) = Regex::new(regex) {
                if let Some(m) = re.find(s) {
                    return Some(&s[m.start()..m.end()]);
                }
            }
            None
        }
    }
}
