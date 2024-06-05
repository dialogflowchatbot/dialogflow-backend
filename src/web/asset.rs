use std::collections::HashMap;

use once_cell::sync::Lazy;

pub(crate) static ASSETS_MAP: Lazy<HashMap<&str, usize>> = Lazy::new(|| {
    HashMap::from([
        (r"/assets/index-DXJ5f3Oq.js", 0),
        (r"/assets/index-X6IkoCLM.css", 1),
        (r"/favicon.ico", 2),
        ("/", 3),
        (r"/index.html", 3),
    ])
});
