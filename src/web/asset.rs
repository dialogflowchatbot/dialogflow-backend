use std::collections::HashMap;

use once_cell::sync::Lazy;

pub(crate) static ASSETS_MAP: Lazy<HashMap<&str, usize>> = Lazy::new(|| {
    HashMap::from([
        (r"/assets/inbound-bot-PJJg_rST.png", 0),
        (r"/assets/index-CFMBpwmb.css", 1),
        (r"/assets/index-CGwDfCl8.js", 2),
        (r"/assets/outbound-bot-EmsLuWRN.png", 3),
        (r"/assets/text-bot-CWb_Poym.png", 4),
        (r"/favicon.ico", 5),
        ("/", 6),
        (r"/index.html", 6),
    ])
});
