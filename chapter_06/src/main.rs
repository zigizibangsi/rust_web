#![warn(clippy::all)]

use warp::Filter;
use warp::http::Method;

use handle_errors::return_error;
use tracing_subscriber::fmt::format::FmtSpan;
mod routes;
mod store;
mod types;

#[tokio::main]
async fn main() {
    // env_logger::init();
    /*
        log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

        log::error!("This is an error");
        log::info!("This is info!");
        log::warn!("This is a warning!");

        let log = warp::log::custom(|info| {
            log::info!(
                "{} {} {} {:?} from {} with {:?}",
                info.method(),
                info.path(),
                info.status(),
                info.elapsed(),
                info.remote_addr().unwrap(),
                info.request_headers(),
            );
        });
    */

    let log_filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "practical_rust_book=info, warp=error".to_owned()); // 1단계 : 로그 수준을 추가한다.

    let store = store::Store::new();
    let store_filter = warp::any().map(move || store.clone());

    // let id_filter = warp::any().map(|| uuid::Uuid::new_v4().to_string());

    tracing_subscriber::fmt()
        // 위에 만든 필터로 어떤 추적을 기록할지 결정한다.
        .with_env_filter(log_filter)
        // 각 범위가 닫힐 때 이벤트를 기록한다.
        // routes 구간에서 사용된다.
        .with_span_events(FmtSpan::CLOSE)
        .init(); // 2단계 : 추적 구독자를 설정한다.

    let cors = warp::cors()
        .allow_any_origin()
        .allow_header("Content-Type")
        .allow_methods(&[Method::PUT, Method::DELETE, Method::POST, Method::GET]);

    let get_questions = warp::get()
        .and(warp::path("questions"))
        .and(warp::path::end())
        .and(warp::query())
        .and(store_filter.clone())
        // .and(id_filter)
        .and_then(routes::question::get_questions)
        .with(warp::trace(|info| {
            tracing::info_span!(
                "get_questions request",
                method = %info.method(),
                path = %info.path(),
                id = %uuid::Uuid::new_v4(),
            )
        })); // 3단계 : 사용자 정의 이벤트에 대한 로깅을 설정한다.

    let add_question = warp::post() // 새로운 변수를 만들어 warp::post로 HTTP POST 요청에 대한 필터를 만든다.
        .and(warp::path("questions")) // 아직은 동일한 최상위 경로 /questions에서 요청을 받는다.
        .and(warp::path::end()) // 경로 정의를 마친다.
        .and(store_filter.clone()) // 이 경로에 저장소를 추가해서 나중에 경로 핸들러에 전달한다.
        .and(warp::body::json()) // 내용을 JSON 으로 추출한다. 추출한 내용은 매개변수로 추가된다.
        .and_then(routes::question::add_question); // 저장소와 추출한 json 값으로 add_question을 실행한다.

    let update_question = warp::put() // 새로운 변수를 만들고 warp::put로 HTTP PUT 요청에 대한 필터를 구성한다.
        .and(warp::path("questions")) // 아직까지는 동일한, 최상위 경로 /questionsfmf Tmsek
        .and(warp::path::param::<String>()) // String 매개변수를 추가하여 /questions/1234 같은 경로에서 동작하도록 한다
        .and(warp::path::end()) // 경로 정의를 끝낸다
        .and(store_filter.clone()) // 이 경로에 저장소를 추가해서 나중에 경로 핸들러로 전달한다
        .and(warp::body::json()) // JSON 내용을 추출해서 매개변수로 추가한다
        .and_then(routes::question::update_question); // 저장소와 JSON을 매개변수로 하여 update_question을 호출한다.

    let delete_question = warp::delete()
        .and(warp::path("questions"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(store_filter.clone())
        .and_then(routes::question::delete_question);

    let add_answer = warp::post()
        .and(warp::path("answers"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and(warp::body::form())
        .and_then(routes::answer::add_answer);

    let routes = get_questions
        .or(add_question)
        .or(update_question)
        .or(delete_question)
        .or(add_answer)
        .with(cors)
        // .with(log)
        .with(warp::trace::request()) // 4단계 : 들어오는 요청에 대한 로깅을 설정한다.
        .recover(return_error);
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
