use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub(crate) struct IntentFormData {
    pub(crate) id: String,
    pub(crate) data: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct Intent {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) keyword_num: usize,
    pub(crate) regex_num: usize,
    pub(crate) phrase_num: usize,
}

impl Intent {
    pub(crate) fn new(intent_name: &str) -> Self {
        // println!("{}", scru128::new().to_u128());
        Intent {
            id: scru128::new_string(),
            name: String::from(intent_name),
            keyword_num: 0,
            regex_num: 0,
            phrase_num: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct IntentDetail {
    pub(crate) intent_idx: usize,
    pub(crate) keywords: Vec<String>,
    pub(crate) regexes: Vec<String>,
    pub(crate) phrases: Vec<String>,
}
