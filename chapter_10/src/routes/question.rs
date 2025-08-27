use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{Level, event, info, instrument};
use warp::http::StatusCode;

use crate::profanity::check_profanity; // 새로 만든 파일에서 내보낸 check_profanity 함수를 임포트한다.
use crate::store::Store;
use crate::types::account::Session; // account 모듈에서 Session 타입을 임포트한다.
use crate::types::pagination::{Pagination, extract_pagination};
use crate::types::question::{NewQuestion, Question};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct APIResponse {
    message: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct BadWord {
    original: String,
    word: String,
    deviations: i64,
    info: i64,
    #[serde(rename = "replacedLen")]
    replaced_len: i64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct BadWordsResponse {
    content: String,
    bad_words_total: i64,
    bad_words_list: Vec<BadWord>,
    censored_content: String,
}

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
    match store
        .get_questions(pagination.limit, pagination.offset)
        .await
    {
        Ok(res) => Ok(warp::reply::json(&res)),
        Err(e) => Err(warp::reject::custom(e)), // 에러의 경우, handle-errors 크레이트에서 정의한 에러 값을 에러 핸들러에 넘긴다.
    }
}

pub async fn add_question(
    session: Session,
    store: Store,
    new_question: NewQuestion,
) -> Result<impl warp::Reply, warp::Rejection> {
    let account_id = session.account_id;
    let title = match check_profanity(new_question.title).await {
        // 함수를 호출하고 퓨처를 기다린 후 Result에 일치시킨다.
        Ok(res) => res,
        Err(e) => return Err(warp::reject::custom(e)),
    };

    let content = match check_profanity(new_question.content).await {
        // 이 작업을 두 번째로 한다. 첫 번째는 title이었다. 이제 질문 자체 안에 있는 금칙어를 검사한다.
        Ok(res) => res,
        Err(e) => return Err(warp::reject::custom(e)),
    };

    let question = NewQuestion {
        title: title,
        content,
        tags: new_question.tags,
    };

    match store.add_question(question, account_id).await {
        Ok(question) => Ok(warp::reply::json(&question)), // 여기까지 왔다면 단순한 문자열과 HTTP 코드 대신에 정확한 질문을 반환한다.
        Err(e) => Err(warp::reject::custom(e)),
    }
}

// pub async fn update_question(
//     id: String,
//     store: Store,
//     question: Question,
// ) -> Result<impl warp::Reply, warp::Rejection> {
//     let title = match check_profanity(question.title).await {
//         Ok(res) => res,
//         Err(e) => return Err(warp::reject::custom(e)),
//     };

//     let content = match check_profanity(question.content).await {
//         Ok(res) => res,
//         Err(e) => return Err(warp::reject::custom(e)),
//     };

//     let question = Question {
//         id: question.id,
//         title,
//         content,
//         tags: question.tags,
//     };
// }

// // tokio spawn 버전
// pub async fn update_question(
//     id: i32,
//     store: Store,
//     question: Question,
// ) -> Result<impl warp::Reply, warp::Rejection> {
//     let title = tokio::spawn(check_profanity(question.title)); // tokio::spawn을 사용하여 기다리지 않고 퓨처를 반환하는 비동기 함수를 래핑한다.
//     let content = tokio::spawn(check_profanity(question.content)); // 질문의 내용에도 동일하게 검사한다.
//     let (title, content) = (title.await.unwrap(), content.await.unwrap()); // 이제 제목에 대한 결과와 내용 확인에 대한 Result를 포함하는 튜플을 반환하여 두 가지를 동시에 실행할 수 있다.

//     if title.is_err() {
//         return Err(warp::reject::custom(title.unwrap_err()));
//     }

//     if content.is_err() {
//         return Err(warp::reject::custom(content.unwrap_err()));
//     } // 두 HTTP 호출이 모두 성공했는지 확인한다.

//     let question = Question {
//         id: question.id,
//         title: title.unwrap(),
//         content: content.unwrap(), // Result를 여기에서 다시 푼다.
//         tags: question.tags,
//     };

//     match store.update_question(question, id).await {
//         Ok(res) => Ok(warp::reply::json(&res)),
//         Err(e) => Err(warp::reject::custom(e)),
//     }
// }

// tokio::join 버전
pub async fn update_question(
    id: i32,
    session: Session, // auth 미들웨어에서 추출하므로 두 번째 매개변수로 Session xkdlqdmf rleogksek.
    store: Store,
    question: Question,
) -> Result<impl warp::Reply, warp::Rejection> {
    let account_id = session.account_id; // account_id를 Session 객체에서 추출하여 다음 함수에 참조로 전달할 수 있도록 한다.
    if store.is_question_owner(id, &account_id).await? {
        // 새로 만든 저장 함수로 해당 질문이 현재 계정으로 생성된 것인지 확인한다.
        let title = check_profanity(question.title);
        let content = check_profanity(question.content);
        let (title, content) = tokio::join!(title, content); // spawn 대신 함수 호출을 개별적으로 래핑할 필요가 없다. join! 매크로 안에서 await 없이 이들을 호출하기만 하면 된다.

        if title.is_ok() && content.is_ok() {
            let question = Question {
                id: question.id,
                title: title.unwrap(),
                content: content.unwrap(),
                tags: question.tags,
            };

            match store.update_question(question, id, account_id).await {
                // 이제 account_id를 저장 함수에 전달하여 데이터베이스 각 항목에 추가된 account_id를 채운다.
                Ok(res) => Ok(warp::reply::json(&res)),
                Err(e) => Err(warp::reject::custom(e)),
            }
        } else {
            Err(warp::reject::custom(
                title.expect_err("Expected API call to have failed here"),
            ))
        }
    } else {
        Err(warp::reject::custom(handle_errors::Error::Unauthorized)) // Session 의 account_id가 데이터베이스의 것과 일치하지 않으면 401 권한 없음 에러를 반환한다.
    }
}

pub async fn delete_question(
    id: i32,
    session: Session,
    store: Store,
) -> Result<impl warp::Reply, warp::Rejection> {
    let account_id = session.account_id;
    if store.is_question_owner(id, &account_id).await? {
        match store.delete_question(id, account_id).await {
            Ok(_) => Ok(warp::reply::with_status(
                format!("Question {} deleted", id),
                StatusCode::OK,
            )),
            Err(e) => Err(warp::reject::custom(e)),
        }
    } else {
        Err(warp::reject::custom(handle_errors::Error::Unauthorized))
    }
}
