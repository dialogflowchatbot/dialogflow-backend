// use jieba_rs::Jieba;
// use simsearch::{SearchOptions, SimSearch};
// use strsim::damerau_levenshtein as a;
// use textdistance::nstr::damerau_levenshtein;
use tokio::runtime::Builder;
// use triple_accel::levenshtein::levenshtein_simd_k;

use dialogflow::web::server::start_app;

fn main() {
    /*
    println!("{}", ((1.0 - 0.8) * 2_f32).ceil() as u32);

    let options = SearchOptions::new().levenshtein(false);
    let mut engine: SimSearch<&'static str> = SimSearch::new_with(options);

    engine.insert("不喜欢1", "我 不喜欢 这个 产品");
    // engine.insert("不喜欢2", "我 讨厌");
    engine.insert("喜欢1", "我 超级 喜欢 另外 一个 产品");
    // engine.insert("喜欢2", "我 爱 这个");
    // engine.insert("喜欢1", "我 喜欢 这个 产品");
    // engine.insert("闲聊", "哈哈哈哈");

    let results: Vec<&'static str> = engine.search("喜欢");
    println!("{:?}", results);

    // assert_eq!(results, &[1]);

    println!("{:?}", damerau_levenshtein("我喜欢", "喜欢"));
    println!("{:?}", damerau_levenshtein("我不喜欢", "喜欢"));
    println!("{:?}", damerau_levenshtein("我喜欢这个产品", "喜欢"));
    println!("{:?}", damerau_levenshtein("我不喜欢这个产品", "喜欢"));

    let token = "我 超级 喜欢 这个 产品";
    let pattern_token = "喜欢";
    let len = std::cmp::max(token.len(), pattern_token.len()) as f64;
    println!("len {}", len);
    let k = ((1.0 - 0.8) * len).ceil() as u32;
    println!("k {}", k);
    println!(
        "zz {:?}",
        levenshtein_simd_k(token.as_bytes(), pattern_token.as_bytes(), 3)
    );

    // println!("{:?}", levenshtein("i like it", "i like"));
    // println!("{:?}", levenshtein("i don't like it", "i like"));

    // let buf = std::io::Cursor::new(b"不喜欢");
    let mut jieba = Jieba::new();
    jieba.add_word("不喜欢", None, None);
    let words = jieba.cut("我不喜欢这个产品", false);
    println!("{:?}", words);

    println!("{:?}", a("我喜欢", "喜欢"));
    println!("{:?}", a("我不喜欢", "喜欢"));
    println!("{:?}", a("我超级喜欢这个产品", "喜欢"));
    println!("{:?}", a("我不喜欢这个产品", "喜欢"));
    println!("{:?}", a("产品", "喜欢"));
    */

    // dialogflow::web::t1();

    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("dialog-flow-system")
        .thread_stack_size(3 * 1024 * 1024)
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    // let (sender, recv) = tokio::sync::oneshot::channel::<()>();
    // runtime.spawn(dialogflow::web::clean_expired_session(recv));
    runtime.block_on(start_app());
}
