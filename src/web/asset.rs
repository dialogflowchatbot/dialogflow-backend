use std::collections::HashMap;

use once_cell::sync::Lazy;

pub(crate) static ASSETS_MAP: Lazy<HashMap<&str, usize>> = Lazy::new(|| {
HashMap::from([
("/assets/browsers-f6aeadcd.png", 0),
("/assets/canvas-dee963ae.png", 1),
("/assets/CollectNode-b510b7be.png", 2),
("/assets/compatible-b1748181.png", 3),
("/assets/conditionNode-010dcdb6.png", 4),
("/assets/diversity-b-35acc628.png", 5),
("/assets/easy-b-90a7b72a.png", 6),
("/assets/externalApiNode-28dd0ff4.png", 7),
("/assets/flow-14ef8935.png", 8),
("/assets/header_bg-9b92bf12.jpg", 9),
("/assets/hero_bg-0a348a9f.jpg", 10),
("/assets/index-a8d4c523.css", 11),
("/assets/index-cc9d124f.js", 12),
("/assets/link-b-5420aaad.png", 13),
("/assets/os-4f42ae1a.png", 14),
("/assets/scenarios-4eff812a.png", 15),
("/assets/step1-a12f6e89.png", 16),
("/assets/step2-02ca4cce.png", 17),
("/assets/step3-aa396807.png", 18),
("/assets/step4-2aa8586e.png", 19),
("/assets/step5-c7889810.png", 20),
("/assets/step6-021d6e36.png", 21),
("/assets/step7-2ebc3d54.png", 22),
("/assets/step8-4ffd1e3d.png", 23),
("/assets/step9-772a025e.png", 24),
("/favicon.ico", 25),
("/", 26),
("/index.html", 26),
])});
