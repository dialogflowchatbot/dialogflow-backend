use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::flow::rt::context::Context;
use crate::flow::rt::dto::{Request, UserInputResult};
use crate::variable::crud as variable;
use crate::variable::dto::VariableType;

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
    HasValue,
    DoesNotHaveValue,
    EmptyString,
    Eq,
    NotEq,
    Contains,
    NotContains,
    NGT,
    NGTE,
    NLT,
    NLTE,
    Timeout,
}

#[derive(
    Copy, Clone, Debug, Deserialize, Serialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
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
    fn get_target_data(&self, req: &Request, ctx: &mut Context) -> String {
        match self.target_data_variant {
            TargetDataVariant::Const => self.target_data.clone(),
            TargetDataVariant::Variable => variable::get_value(&self.target_data, req, ctx),
        }
    }
    pub(in crate::flow::rt) fn compare(&self, req: &Request, ctx: &mut Context) -> bool {
        // let target_data = match self.target_data_variant {
        //     TargetDataVariant::Const => self.target_data.clone(),
        //     TargetDataVariant::Variable => variable::get_value(&self.target_data, req, ctx),
        // };
        // println!("{} {}", &target_data, &req.user_input);
        match self.condition_type {
            ConditionType::UserInput => match self.compare_type {
                CompareType::Eq => self.get_target_data(req, ctx).eq(&req.user_input),
                CompareType::Contains => req.user_input.contains(&self.get_target_data(req, ctx)),
                CompareType::Timeout => UserInputResult::Timeout == req.user_input_result,
                _ => false,
            },
            ConditionType::UserIntent => {
                // println!("{} {}", &target_data, req.user_input_intent.is_some());
                req.user_input_intent.is_some()
                    && self
                        .get_target_data(req, ctx)
                        .eq(req.user_input_intent.as_ref().unwrap())
            }
            ConditionType::FlowVariable => match self.compare_type {
                CompareType::HasValue => {
                    if let Ok(op) = variable::get(&self.ref_data) {
                        if let Some(v) = op {
                            v.get_value(req, ctx).is_some()
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                CompareType::DoesNotHaveValue => {
                    if let Ok(op) = variable::get(&self.ref_data) {
                        if let Some(v) = op {
                            v.get_value(req, ctx).is_none()
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                }
                CompareType::EmptyString => {
                    if let Ok(op) = variable::get(&self.ref_data) {
                        if let Some(v) = op {
                            if v.var_type == VariableType::Num {
                                false
                            } else {
                                let val = v.get_value(req, ctx);
                                val.is_none() || val.as_ref().unwrap().val_to_string().is_empty()
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                CompareType::Eq => {
                    if let Ok(op) = variable::get(&self.ref_data) {
                        if let Some(v) = op {
                            if let Some(val) = v.get_value(req, ctx) {
                                val.val_to_string().eq(&self.get_target_data(req, ctx))
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                CompareType::NotEq => {
                    if let Ok(op) = variable::get(&self.ref_data) {
                        if let Some(v) = op {
                            if let Some(val) = v.get_value(req, ctx) {
                                !val.val_to_string().eq(&self.get_target_data(req, ctx))
                            } else {
                                true
                            }
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                }
                CompareType::Contains => {
                    if let Ok(op) = variable::get(&self.ref_data) {
                        if let Some(v) = op {
                            if v.var_type == VariableType::Num {
                                false
                            } else {
                                if let Some(val) = v.get_value(req, ctx) {
                                    val.val_to_string()
                                        .find(&self.get_target_data(req, ctx))
                                        .is_some()
                                } else {
                                    true
                                }
                            }
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                }
                CompareType::NotContains => {
                    if let Ok(op) = variable::get(&self.ref_data) {
                        if let Some(v) = op {
                            if v.var_type == VariableType::Num {
                                false
                            } else {
                                if let Some(val) = v.get_value(req, ctx) {
                                    val.val_to_string()
                                        .find(&self.get_target_data(req, ctx))
                                        .is_none()
                                } else {
                                    true
                                }
                            }
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                }
                CompareType::NGT => {
                    if let Ok(op) = variable::get(&self.ref_data) {
                        if let Some(v) = op {
                            if v.var_type == VariableType::Str {
                                false
                            } else {
                                if let Some(val) = v.get_value(req, ctx) {
                                    if let Ok(n1) = val.val_to_string().parse::<f64>() {
                                        // println!("get_target_data {} {:?} |{}|", self.target_data, self.target_data_variant, self.get_target_data(req, ctx));
                                        if let Ok(n2) =
                                            self.get_target_data(req, ctx).parse::<f64>()
                                        {
                                            // println!("{} {}", n1, n2);
                                            n1 > n2
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                CompareType::NGTE => {
                    if let Ok(op) = variable::get(&self.ref_data) {
                        if let Some(v) = op {
                            if v.var_type == VariableType::Str {
                                false
                            } else {
                                if let Some(val) = v.get_value(req, ctx) {
                                    if let Ok(n1) = val.val_to_string().parse::<f64>() {
                                        if let Ok(n2) =
                                            self.get_target_data(req, ctx).parse::<f64>()
                                        {
                                            n1 >= n2
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                CompareType::NLT => {
                    if let Ok(op) = variable::get(&self.ref_data) {
                        if let Some(v) = op {
                            if v.var_type == VariableType::Str {
                                false
                            } else {
                                if let Some(val) = v.get_value(req, ctx) {
                                    if let Ok(n1) = val.val_to_string().parse::<f64>() {
                                        if let Ok(n2) =
                                            self.get_target_data(req, ctx).parse::<f64>()
                                        {
                                            n1 < n2
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                CompareType::NLTE => {
                    if let Ok(op) = variable::get(&self.ref_data) {
                        if let Some(v) = op {
                            if v.var_type == VariableType::Str {
                                false
                            } else {
                                if let Some(val) = v.get_value(req, ctx) {
                                    if let Ok(n1) = val.val_to_string().parse::<f64>() {
                                        if let Ok(n2) =
                                            self.get_target_data(req, ctx).parse::<f64>()
                                        {
                                            n1 <= n2
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                // let mut n = false;
                // if let Ok(r) = variable::get(&self.ref_data) {
                //     if let Some(ref_v) = r {
                //         if let Some(val) = ref_v.get_value(req, ctx) {
                //             n = val.val_to_string().eq(&target_data);
                //         }
                //     }
                // }
                _ => false,
            },
            ConditionType::CustomJavascript => todo!(),
            ConditionType::CustomRegex => {
                if let Ok(re) = Regex::new(&self.get_target_data(req, ctx)) {
                    return re.is_match(&req.user_input);
                }
                false
            }
        }
    }
}
