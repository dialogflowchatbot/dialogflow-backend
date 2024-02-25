use std::collections::HashMap;

use once_cell::sync::Lazy;

pub(crate) static ASSETS_MAP: Lazy<HashMap<&str, usize>> = Lazy::new(|| {
HashMap::from([
("/assets/canvas-ebZo_yLf.js", 0),
("/assets/index-MTyOoNFh.js", 1),
("/assets/index-naI8Vgvx.css", 2),
("/assets/__commonjsHelpers__-w40geAFS.js", 3),
("/favicon.ico", 4),
("/", 5),
("/index.html", 5),
])});
