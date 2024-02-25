// use std::net::SocketAddr;
use std::vec::Vec;

use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use colored::Colorize;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use super::asset::ASSETS_MAP;
use crate::external::http::crud as http;
use crate::flow::mainflow::crud as mainflow;
use crate::flow::rt::facade as rt;
use crate::flow::subflow::crud as subflow;
use crate::intent::crud as intent;
use crate::man::settings;
use crate::result::Error;
use crate::variable::crud as variable;

//https://stackoverflow.com/questions/27840394/how-can-a-rust-program-access-metadata-from-its-cargo-package
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");
static VERSION_NUM: Lazy<u64> = Lazy::new(|| convert_version(VERSION));

const ASSETS: &[(&[u8], &str)] = &include!("asset.txt");

pub(crate) static IS_EN: Lazy<bool> = Lazy::new(|| {
    let language = get_lang();
    // println!("Your OS language is: {}", language);
    language[0..2].eq("en")
});

// https://doc.rust-lang.org/reference/conditional-compilation.html
#[cfg(windows)]
fn get_lang() -> String {
    let mut v = [0u16; windows::Win32::System::SystemServices::LOCALE_NAME_MAX_LENGTH as usize];
    unsafe {
        let l = windows::Win32::Globalization::GetUserDefaultLocaleName(&mut v) as usize;
        String::from_utf16(&v[0..l]).unwrap()
        // windows::Win32::Globalization::GetUserDefaultLangID().to_string()
    }
}

#[cfg(not(windows))]
fn get_lang() -> String {
    std::env::var("LANG").unwrap_or(String::from("zh_CN"))
}

// fn invalid_ip_msg(addr: &String) -> String {
//     format!("Invalid listening addr {}, please reset the configuration parameters by adding the startup parameter: {}",
//     addr.bright_red(), "-rs".bright_yellow())
// }

pub async fn start_app() {
    let settings = {
        let mut s = crate::db::init().await.expect("Initialize database failed");
        for argument in std::env::args() {
            if argument.eq("-rs") {
                s = settings::Settings::default();
                settings::save_settings(&s).expect("Reset settings failed");
                break;
            }
        }
        s
    };

    #[cfg(target_os = "windows")]
    let _ = colored::control::set_virtual_terminal(true).unwrap();

    log::info!(
        "  -->  {} {}{}:{}",
        if *IS_EN {
            "Please open a browser and visit"
        } else {
            "请用浏览器访问"
        },
        "http://".bright_green(),
        settings.ip.bright_green(),
        settings.port.to_string().blue()
    );
    log::info!("Current version: {}", VERSION);
    log::info!("Visiting https://dialogflowchatbot.github.io/ for the latest releases");

    log::info!(
        "  -->  Press {} to terminate this application",
        "Ctrl+C".bright_red()
    );

    let (sender, recv) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(crate::flow::rt::context::clean_expired_session(
        recv,
        settings.max_session_duration_min,
    ));

    let r: Router = gen_router();
    let app = r.fallback(fallback);
    let addr = format!("{}:{}", settings.ip, settings.port);
    // let socket_addr: SocketAddr = addr.parse().expect(&invalid_ip_msg(&addr));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    // let addr = SocketAddr::from((settings.ip, settings.port));
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(sender))
        .await
        .unwrap();
}

fn gen_router() -> Router {
    Router::new()
        .route(
            "/intent",
            get(intent::list).post(intent::add).delete(intent::remove),
        )
        .route("/intent/detail", get(intent::detail))
        .route(
            "/intent/keyword",
            post(intent::add_keyword).delete(intent::remove_keyword),
        )
        .route(
            "/intent/regex",
            post(intent::add_regex).delete(intent::remove_regex),
        )
        .route(
            "/intent/phrase",
            post(intent::add_phrase).delete(intent::remove_phrase),
        )
        .route(
            "/variable",
            get(variable::list)
                .post(variable::add)
                .delete(variable::delete),
        )
        .route(
            "/mainflow",
            get(mainflow::list)
                .post(mainflow::new)
                .put(mainflow::save)
                .delete(mainflow::delete),
        )
        .route("/mainflow/release", get(subflow::release))
        .route(
            "/subflow",
            get(subflow::list)
                .post(subflow::save)
                .delete(subflow::delete),
        )
        .route("/subflow/simple", get(subflow::simple_list))
        .route("/subflow/new", post(subflow::new))
        .route("/external/http", get(http::list))
        .route(
            "/external/http/:id",
            get(http::detail).post(http::save).delete(http::remove),
        )
        .route(
            "/management/settings",
            get(settings::get).post(settings::save),
        )
        .route("/flow/answer", post(rt::answer))
        .route("/version.json", get(version))
        .route("/check-new-version.json", get(check_new_version))
        // .route("/o", get(flow::output))
        .layer(
            CorsLayer::new()
                .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
                .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PUT]),
        )
}

// https://docs.rs/axum/0.6.18/axum/response/index.html

async fn fallback(uri: Uri) -> Response {
    let v = ASSETS_MAP.get(uri.path());
    if v.is_some() {
        let idx = v.unwrap();
        let d = ASSETS[*idx];
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, d.1.parse().unwrap());
        headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
        (StatusCode::OK, headers, d.0).into_response()
    } else {
        (StatusCode::NOT_FOUND, format!("Not Found: {}", uri.path())).into_response()
    }
}

fn convert_version(ver: &str) -> u64 {
    let arr: Vec<&str> = ver.split('.').collect();
    let mut v = String::with_capacity(VERSION.len() + 4);
    v.push_str(arr[0]);
    if arr[1].len() == 1 {
        v.push_str("00");
    } else if arr[1].len() == 2 {
        v.push_str("0");
    }
    v.push_str(arr[1]);
    if arr[2].len() == 1 {
        v.push_str("00");
    } else if arr[2].len() == 2 {
        v.push_str("0");
    }
    v.push_str(arr[2]);
    // log::info!("vernum={}", &v);
    v.parse().expect("Wrong version")
}

async fn version() -> impl IntoResponse {
    let mut v = String::with_capacity(15);
    let _ = v.push('"');
    v.push_str(VERSION);
    let _ = v.push('"');
    v
}

async fn check_new_version() -> impl IntoResponse {
    let r = reqwest::get(
        "https://dialogflowchatbot.github.io/check-new-version.json",
    )
    .await;
    if let Err(e) = r {
        return to_res(Err(Error::NetworkConnectTimeout(e)));
    }
    r.unwrap().text().await.map_or_else(
        |e| to_res(Err(Error::NetworkReadTimeout(e))),
        |s| {
            #[derive(Debug, Deserialize, Serialize)]
            struct VersionInfo {
                version: String,
                changelog: Vec<String>,
            }
            let obj: core::result::Result<VersionInfo, _> = serde_json::from_str(&s);
            if let Err(e) = obj {
                return to_res(Err(Error::InvalidJsonStructure(e)));
            }
            let v = obj.unwrap();
            if convert_version(&v.version) > *VERSION_NUM {
                to_res(Ok(Some(v)))
            } else {
                to_res(Ok(None))
            }
        },
    )
}

async fn shutdown_signal(sender: tokio::sync::oneshot::Sender<()>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    match sender.send(()) {
        Ok(_) => {}
        Err(_) => log::info!("中断 ctx 失败"),
    };

    let m = if *IS_EN {
        "This program has been terminated"
    } else {
        "应用已退出"
    };
    log::info!("{}", m);
}

#[derive(Serialize)]
struct ResponseData<D> {
    pub(crate) status: u16,
    pub(crate) data: Option<D>,
    pub(crate) err: Option<Error>,
}

pub(crate) fn to_res<D>(r: Result<D, Error>) -> impl IntoResponse
where
    D: serde::Serialize + 'static,
{
    // let now = std::time::Instant::now();
    let data = match r {
        Ok(d) => {
            let res = ResponseData {
                status: StatusCode::OK.as_u16(),
                data: Some(&d),
                err: None,
            };
            serde_json::to_string(&res).unwrap()
            // simd_json::to_string(&res).unwrap()
        }
        Err(e) => {
            let res: ResponseData<D> = ResponseData {
                status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                data: None,
                err: Some(e),
            };
            serde_json::to_string(&res).unwrap()
            // simd_json::to_string(&res).unwrap()
        }
    };
    // log::info!("serialize used time:{:?}", now.elapsed());
    let mut header_map = HeaderMap::new();
    header_map.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    (StatusCode::OK, header_map, data)
}
