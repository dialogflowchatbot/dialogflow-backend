use std::collections::HashMap;

use once_cell::sync::Lazy;

pub(crate) static ASSETS_MAP: Lazy<HashMap<&str, usize>> = Lazy::new(|| {
HashMap::from([
("/assets/index-93NIe4JJ.css", 0),
("/assets/index-9D092EYe.js", 1),
("/favicon.ico", 2),
("/", 3),
("/index.html", 3),
])});
