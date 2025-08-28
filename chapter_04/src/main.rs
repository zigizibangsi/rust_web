use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use warp::{
    Filter, Rejection, Reply, filters::body::BodyDeserializeError, filters::cors::CorsForbidden,
    http::Method, http::StatusCode, reject::Reject,
};

use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Serialize, Debug, Deserialize, Clone)]
struct Question {
    id: QuestionId,
    title: String,
    content: String,
    tags: Option<Vec<String>>,
}

#[derive(Serialize, Debug, Clone, Eq, Hash, Deserialize, PartialEq)]
struct QuestionId(String);

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
struct AnswerId(String);

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Answer {
    id: AnswerId,
    content: String,
    question_id: QuestionId,
}

#[derive(Clone)]
struct Store {
    questions: Arc<RwLock<HashMap<QuestionId, Question>>>,
    answers: Arc<RwLock<HashMap<AnswerId, Answer>>>,
}

impl Store {
    fn new() -> Self {
        Store {
            questions: Arc::new(RwLock::new(Self::init())),
            answers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn init() -> HashMap<QuestionId, Question> {
        let file = include_str!("../questions.json");
        serde_json::from_str(file).expect("can't read questions.json")
    }
}

#[derive(Debug)]
struct InvalidId;
impl Reject for InvalidId {}

#[derive(Debug)]
struct Pagination {
    start: usize,
    end: usize,
}

fn extract_pagination(params: HashMap<String, String>) -> Result<Pagination, Error> {
    if params.contains_key("start") && params.contains_key("end") {
        // HashMap에 .contains 메서드를 써서 두 매개변수가 모두 있는지 확인한다.
        return Ok(Pagination {
            // 두 매개변수가 모두 있으면 Result를 반환(return Ok()) 한다. 바로 돌아가기 위해서 return 키워드를 사용한다. // 새로운 Pagination 객체를 만들고 start와 end 번호를 설정한다.
            start: params
                .get("start") // HashMap의 .get 메서드로 옵션을 반환한다. 해당 메서드로 키가 확실히 존재하는지 보증할 수 없기 때문이다. 몇 줄 전에 HashMap에 매개변수가 두 개인지 먼저 확인했으므로 안전하지 않은 .unwrap을 사용해도 된다. HashMap의 &str 값을 usize 정수 타입으로 파싱한다. 파싱 결과로 Result를 반환하며, 값을 풀어 내거나 파싱에 실패했을 때는 .map_err와 줄 끝의 물음표를 이용해 에러를 반환한다.
                .unwrap()
                .parse::<usize>()
                .map_err(Error::ParseError)?,
            end: params
                .get("end")
                .unwrap()
                .parse::<usize>()
                .map_err(Error::ParseError)?,
        });
    }
    Err(Error::MissingParameters) // 그렇지 않은 경우 if 절은 실행되지 않고 바로 Err로 이동하여 사용자 정의 MissingParameters 에러를 반환한다. 여기서 이중 콜론(::)을 사용하여 Error 열거 타입에서 접근한다.
}

async fn get_questions(
    params: HashMap<String, String>,
    store: Store,
) -> Result<impl warp::Reply, warp::Rejection> {
    if !params.is_empty() {
        let pagination = extract_pagination(params)?;
        let res: Vec<Question> = store.questions.read().await.values().cloned().collect();
        let res = &res[pagination.start..pagination.end];
        Ok(warp::reply::json(&res))
    } else {
        let res: Vec<Question> = store.questions.read().await.values().cloned().collect();
        Ok(warp::reply::json(&res))
    }
}

async fn add_question(
    store: Store,
    question: Question,
) -> Result<impl warp::Reply, warp::Rejection> {
    store
        .questions
        .write()
        .await
        .insert(question.id.clone(), question);

    Ok(warp::reply::with_status("Question added", StatusCode::OK))
}

#[derive(Debug)]
enum Error {
    ParseError(std::num::ParseIntError),
    MissingParameters,
    QuestionNotFound,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::ParseError(ref err) => {
                write!(f, "Cannot parse parameter: {}", err)
            }
            Error::MissingParameters => {
                write!(f, "Missing parameters")
            }
            Error::QuestionNotFound => write!(f, "Question not found"),
        }
    }
}

impl Reject for Error {}

async fn return_error(r: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(error) = r.find::<Error>() {
        Ok(warp::reply::with_status(
            error.to_string(),
            StatusCode::RANGE_NOT_SATISFIABLE,
        ))
    } else if let Some(error) = r.find::<CorsForbidden>() {
        Ok(warp::reply::with_status(
            error.to_string(),
            StatusCode::FORBIDDEN,
        ))
    } else if let Some(error) = r.find::<BodyDeserializeError>() {
        Ok(warp::reply::with_status(
            error.to_string(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ))
    } else if let Some(InvalidId) = r.find() {
        Ok(warp::reply::with_status(
            "No valid ID presented".to_string(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ))
    } else {
        Ok(warp::reply::with_status(
            "Route not found".to_string(),
            StatusCode::NOT_FOUND,
        ))
    }
}

async fn update_question(
    id: String,
    store: Store,
    question: Question,
) -> Result<impl warp::Reply, warp::Rejection> {
    match store.questions.write().await.get_mut(&QuestionId(id)) {
        Some(q) => *q = question,
        None => return Err(warp::reject::custom(Error::QuestionNotFound)),
    }
    Ok(warp::reply::with_status("Question updated", StatusCode::OK))
}

async fn delete_question(id: String, store: Store) -> Result<impl warp::Reply, warp::Rejection> {
    match store.questions.write().await.remove(&QuestionId(id)) {
        Some(_) => return Ok(warp::reply::with_status("Question deleted", StatusCode::OK)),
        None => return Err(warp::reject::custom(Error::QuestionNotFound)),
    }
}

async fn add_answer(
    store: Store,
    params: HashMap<String, String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let answer = Answer {
        id: AnswerId("1".to_string()),
        content: params.get("content").unwrap().to_string(),
        question_id: QuestionId(params.get("questionId").unwrap().to_string()),
    };

    store
        .answers
        .write()
        .await
        .insert(answer.id.clone(), answer);

    Ok(warp::reply::with_status("Answer added", StatusCode::OK))
}

#[tokio::main]
async fn main() {
    let store = Store::new();
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
        .and_then(get_questions);

    let add_question = warp::post() // 새로운 변수를 만들어 warp::post로 HTTP POST 요청에 대한 필터를 만든다.
        .and(warp::path("questions")) // 아직은 동일한 최상위 경로 /questions에서 요청을 받는다.
        .and(warp::path::end()) // 경로 정의를 마친다.
        .and(store_filter.clone()) // 이 경로에 저장소를 추가해서 나중에 경로 핸들러에 전달한다.
        .and(warp::body::json()) // 내용을 JSON 으로 추출한다. 추출한 내용은 매개변수로 추가된다.
        .and_then(add_question); // 저장소와 추출한 json 값으로 add_question을 실행한다.

    let update_question = warp::put() // 새로운 변수를 만들고 warp::put로 HTTP PUT 요청에 대한 필터를 구성한다.
        .and(warp::path("questions")) // 아직까지는 동일한, 최상위 경로 /questionsfmf Tmsek
        .and(warp::path::param::<String>()) // String 매개변수를 추가하여 /questions/1234 같은 경로에서 동작하도록 한다
        .and(warp::path::end()) // 경로 정의를 끝낸다
        .and(store_filter.clone()) // 이 경로에 저장소를 추가해서 나중에 경로 핸들러로 전달한다
        .and(warp::body::json()) // JSON 내용을 추출해서 매개변수로 추가한다
        .and_then(update_question); // 저장소와 JSON을 매개변수로 하여 update_question을 호출한다.

    let delete_question = warp::delete()
        .and(warp::path("questions"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(store_filter.clone())
        .and_then(delete_question);

    let add_answer = warp::post()
        .and(warp::path("answers"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and(warp::body::form())
        .and_then(add_answer);

    let routes = get_questions
        .or(add_question)
        .or(update_question)
        .or(delete_question)
        .or(add_answer)
        .with(cors)
        .recover(return_error);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
