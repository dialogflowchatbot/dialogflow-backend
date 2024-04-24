use std::collections::HashMap;

use once_cell::sync::Lazy;

pub(crate) static ASSETS_MAP: Lazy<HashMap<&str, usize>> = Lazy::new(|| {
    HashMap::from([
        (r"/assets/index-dHLMxLrC.css", 0),
        (r"/assets/index-J5BfAI_8.js", 1),
        (r"/favicon.ico", 2),
        ("/", 3),
        (r"/index.html", 3),
    ])
});
