use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::flow::rt::context::Context;
use crate::flow::rt::dto::{Request, UserInputResult};
use crate::variable::crud as variable;

#[derive(
    Clone, Copy, Deserialize, Serialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
#[archive(compare(PartialEq), check_bytes)]
pub(crate) enum ConditionType {
    UserInput,
    UserIntent,
    FlowVariable,
    CustomJavascript,
    CustomRegex,
}

#[derive(
    Clone, Copy, Deserialize, Serialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
#[archive(compare(PartialEq), check_bytes)]
pub(crate) enum CompareType {
    Eq,
    NotEq,
    Contains,
    NotContains,
    Timeout,
}

#[derive(Clone, Deserialize, Serialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[archive(compare(PartialEq), check_bytes)]
pub(crate) enum TargetDataVariant {
    Const,
    Variable,
}

#[derive(Clone, Deserialize, Serialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[archive(compare(PartialEq), check_bytes)]
pub(crate) struct ConditionData {
    pub(crate) condition_type: ConditionType,
    pub(crate) compare_type: CompareType,
    pub(crate) ref_data: String,
    pub(crate) target_data: String,
    pub(crate) target_data_variant: TargetDataVariant,
}

impl ConditionData {
    pub(in crate::flow::rt) fn compare(&self, req: &Request, ctx: &mut Context) -> bool {
        let target_data = match self.target_data_variant {
            TargetDataVariant::Const => self.target_data.clone(),
            TargetDataVariant::Variable => variable::get_value(&self.target_data, req, ctx),
        };
        // println!("{} {}", &target_data, &req.user_input);
        match self.condition_type {
            ConditionType::UserInput => match self.compare_type {
                CompareType::Eq => target_data.eq(&req.user_input),
                CompareType::Contains => req.user_input.contains(&target_data),
                CompareType::Timeout => UserInputResult::Timeout == req.user_input_result,
                _ => false,
            },
            ConditionType::UserIntent => {
                // println!("{} {}", &target_data, req.user_input_intent.is_some());
                req.user_input_intent.is_some()
                    && target_data.eq(req.user_input_intent.as_ref().unwrap())
            }
            ConditionType::FlowVariable => {
                let mut n = false;
                if let Ok(r) = variable::get(&self.ref_data) {
                    if let Some(ref_v) = r {
                        if let Some(val) = ref_v.get_value(req, ctx) {
                            n = val.val_to_string().eq(&target_data);
                        }
                    }
                }
                n
            }
            ConditionType::CustomJavascript => todo!(),
            ConditionType::CustomRegex => {
                if let Ok(re) = Regex::new(&target_data) {
                    return re.is_match(&req.user_input);
                }
                false
            }
        }
    }
}
