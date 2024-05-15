use std::collections::HashMap;

use once_cell::sync::Lazy;

pub(crate) static ASSETS_MAP: Lazy<HashMap<&str, usize>> = Lazy::new(|| {
    HashMap::from([
        (r"/assets/index-Cb6_iPA-.css", 0),
        (r"/assets/index-D0EGhxTc.js", 1),
        (r"/favicon.ico", 2),
        ("/", 3),
        (r"/index.html", 3),
    ])
});
