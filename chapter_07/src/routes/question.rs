use std::collections::HashMap;
use tracing::{Level, event, info, instrument};
use warp::http::StatusCode;

use crate::store::Store;
use crate::types::pagination::{Pagination, extract_pagination};
use crate::types::question::{Question, QuestionId};

use handle_errors::Error;

#[instrument]
pub async fn get_questions(
    params: HashMap<String, String>,
    store: Store,
) -> Result<impl warp::Reply, warp::Rejection> {
    event!(target: "practical_rust_book", Level::INFO, "querying questions");
    let mut pagination = Pagination::default(); // 기본 매개변수 Pagination 값을 가지는 가변 변수를 만든다.

    if !params.is_empty() {
        event!(Level::INFO, pagination = true);
        let pagination = extract_pagination(params)?; // 페이지 매기기 객체(pagination object)가 비어있지 않은 경우, 위 가변 변수의 값을 클라이언트가 전달한 Pagination 값으로 대체한다.
    }
    info!(pagination = false);
    let res: Vec<Question> = match store
        .get_questions(pagination.limit, pagination.offset)
        .await
    {
        Ok(res) => res,
        Err(e) => {
            return Err(warp::reject::custom(Error::DatabaseQueryError(e))); // 에러의 경우, handle-errors 크레이트에서 정의한 에러 값을 에러 핸들러에 넘긴다.
        }
    };

    Ok(warp::reply::json(&res))
}

pub async fn add_question(
    store: Store,
    new_question: NewQuestion,
) -> Result<impl warp::Reply, warp::Rejection> {
    if let Err(e) = store.add_question(new_question).await {
        return Err(warp::reject::custom(Error::DatabaseQueryError(e)));
    }
    Ok(warp::reply::with_status("Question added", StatusCode::OK))
}

pub async fn update_question(
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

pub async fn delete_question(
    id: String,
    store: Store,
) -> Result<impl warp::Reply, warp::Rejection> {
    match store.questions.write().await.remove(&QuestionId(id)) {
        Some(_) => return Ok(warp::reply::with_status("Question deleted", StatusCode::OK)),
        None => return Err(warp::reject::custom(Error::QuestionNotFound)),
    }
}
