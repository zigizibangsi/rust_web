use serde::Serialize;
use std::io::{Error, ErrorKind};
use std::str::FromStr;

use warp::{filters::cors::CorsForbidden,http::Method, http::StatusCode, reject::Reject, Filter, Rejection, Reply };

#[derive(Debug, Serialize)]
struct Question {
    id: QuestionId,
    title: String,
    content: String,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct QuestionId(String);

impl Question {
    fn new(id: QuestionId, title: String, content: String, tags: Option<Vec<String>>) -> Self {
        Question {
            id,
            title,
            content,
            tags,
        }
    }
}

impl std::fmt::Display for Question {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}, title: {}, content: {}, tags: {:?}",
            self.id, self.title, self.content, self.tags
        )
    }
}

impl std::fmt::Display for QuestionId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "id: {}", self.0)
    }
}

impl FromStr for QuestionId {
    type Err = std::io::Error;

    fn from_str(id: &str) -> Result<Self, Self::Err> {
        match id.is_empty() {
            false => Ok(QuestionId(id.to_string())),
            true => Err(Error::new(ErrorKind::InvalidInput, "No id provided")),
        }
    }
}

#[derive(Debug)]
struct InvalidId;
impl Reject for InvalidId {}

// Warp가 사용할 수 있게 회신과 거부를 반환하는 첫 번째 경로 핸들러를 만든다.
async fn get_questions() -> Result<impl warp::Reply, warp::Rejection> {
    let question = Question::new( // 요청하는 클라이언트에 반환할 새로운 question을 생성한다.
        QuestionId::from_str("1").expect("No id provided"),
        "First Question".to_string(),
        "Content of question".to_string(),
        Some(vec!["faq".to_string()]),
    );
    
    match question.id.0.parse::<i32>() {
        Err(_) => Err(warp::reject::custom(InvalidId)),
        Ok(_) => Ok(warp::reply::json(&question)),
    }
}

async fn return_error(r: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(error) = r.find::<CorsForbidden>() {
        Ok(warp::reply::with_status(
            error.to_string(),
            StatusCode::FORBIDDEN,
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

#[tokio::main]
async fn main() {
    let cors = warp::cors()
        .allow_any_origin()
        //.allow_header("content-type")
        .allow_header("not-in-the-request")
        .allow_methods(&[Method::PUT, Method::DELETE, Method::POST, Method::GET]);

    // 하나 이상의 필터를 결합하는 Warp의 .and 함수를 사용해 큰 필터를 하나 생성하고 get_items에 할당한다.
    let get_items = warp::get() 
        // path::end를 써서 정확히 /questions(예를 들어 /questions/further/params 같은 것은 안 됨)에서만 수신을 받겠다고 신호를 보낸다.
        .and(warp::path("questions")) 
        .and(warp::path::end())
        .and_then(get_questions)
        .recover(return_error);

    let routes = get_items.with(cors).recover(return_error); // 나중의 편의를 위해 경로 변수 routes를 정의한다.

    warp::serve(routes) // routes 필터를 Warp의 serve 메서드로 전달하고 서버를 시작한다.
        .run(([127, 0, 0, 1], 3030))
        .await;
}
