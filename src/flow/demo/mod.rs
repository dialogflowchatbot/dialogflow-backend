pub(crate) const DEMO_COLLECT: &str = include_str!("demo_collect.txt");
pub(crate) const DEMO_NOTIFY: &str = include_str!("demo_notify.txt");
pub(crate) const DEMO_REPAY: &str = include_str!("demo_repay.txt");
pub(crate) const DEMO_COLLECT_EN: &str = include_str!("demo_collect_en.txt");
pub(crate) const DEMO_NOTIFY_EN: &str = include_str!("demo_notify_en.txt");
pub(crate) const DEMO_REPAY_EN: &str = include_str!("demo_repay_en.txt");

use crate::web::server;

pub(crate) fn get_demo<'a>(name: &'a str) -> Option<&'static str> {
    let is_en = *server::IS_EN;
    if name.eq("demo-collect") {
        Some(if is_en { DEMO_COLLECT } else { DEMO_COLLECT_EN })
    } else if name.eq("demo-notify") {
        Some(if is_en { DEMO_NOTIFY } else { DEMO_NOTIFY_EN })
    } else if name.eq("demo-repay") {
        Some(if is_en { DEMO_REPAY } else { DEMO_REPAY_EN })
    } else {
        None
    }
}
