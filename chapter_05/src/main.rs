use warp::Filter;
use warp::http::Method;

use handle_errors::return_error;
mod routes;
mod store;
mod types;

#[tokio::main]
async fn main() {
    let store = store::Store::new();
    let store_filter = warp::any().map(move || store.clone());

    let cors = warp::cors()
        .allow_any_origin()
        .allow_header("Content-Type")
        .allow_methods(&[Method::PUT, Method::DELETE, Method::POST, Method::GET]);

    let get_questions = warp::get()
        .and(warp::path("questions"))
        .and(warp::path::end())
        .and(warp::query())
        .and(store_filter.clone())
        .and_then(routes::question::get_questions);

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
        .recover(return_error);
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
