use argon2::{self, Config}; // argon2 해싱 알고리즘의 구현을 임포트한다.
use chrono::prelude::*;

use rand::Rng; // rand 크레이트의 도움을 받아 임의의 솔트를 만든다.
use std::future;
use warp::Filter;
use warp::http::StatusCode;

use crate::store::Store;
use crate::types::account::{Account, AccountId, Session}; // 토큰을 생성하는 데 사용하므로 AccountId를 임포트한다.

pub fn verify_token(token: String) -> Result<Session, handle_errors::Error> {
    let token = paseto::tokens::validate_local_token(
        &token,
        None,
        &"RANDOM WORDS WINTER MACINTOSH PC".as_bytes(),
        &paseto::tokens::TimeBackend::Chrono,
    )
    .map_err(|_| handle_errors::Error::CannotDecryptToken)?;

    serde_json::from_value::<Session>(token).map_err(|_| handle_errors::Error::CannotDecryptToken)
}

pub async fn register(store: Store, account: Account) -> Result<impl warp::Reply, warp::Rejection> {
    let hashed_password = hash_password(account.password.as_bytes()); // 비밀번호를 바이트 배열로 바꾼 후 새로 만든 해시 함수로 전달한다.

    let account = Account {
        id: account.id,
        email: account.email,
        password: hashed_password, // 데이터베이스에 넣을 용도로 사용자가 입력한 비밀번호(평문) 대신 해시된(그리고 솔트를 추가한) 버전을 사용한다.
    };

    match store.add_account(account).await {
        Ok(_) => Ok(warp::reply::with_status("Account added", StatusCode::OK)),
        Err(e) => Err(warp::reject::custom(e)),
    }
}

pub fn hash_password(password: &[u8]) -> String {
    // 해시 함수는 문자열을 반환하며, 해당 문자열은 평문 비밀번호의 해시된 버전이다.
    let salt = rand::thread_rng().r#gen::<[u8; 32]>(); // rand 함수는 32바이트 크기의 난수를 만들어 슬라이스로 저장한다.
    let config = Config::default(); // argon2는 구성에 따라 다르며, 우리는 기본 설정을 사용한다.
    argon2::hash_encoded(password, &salt, &config).unwrap() // password, salt, config를 사용해서 평문 비밀번호를 해시한다.
}

pub async fn login(store: Store, login: Account) -> Result<impl warp::Reply, warp::Rejection> {
    // 경로 핸들러가 저장소와 로그인 객체를 전달 받을 것으로 가정한다.
    match store.get_account(login.email).await {
        // 먼저 사용자가 데이터베이스에 존재하는지 검사한다.
        Ok(account) => match verify_password(&account.password, login.password.as_bytes()) {
            // 사용자가 존재한다면 비밀번호가 맞는지 검증한다.
            Ok(verified) => {
                // 검증 절차가 성공(라이브러리가 실패하지 않음)한 경우라면 다음을 실행한다.
                if verified {
                    // 비밀번호가 실제로 확인되었는지 검사한다.
                    Ok(warp::reply::json(&issue_token(
                        // 그리고 토큰을 만들어 AccountId에 넣는다.
                        account.id.expect("id not found"),
                    )))
                } else {
                    Err(warp::reject::custom(handle_errors::Error::WrongPassword)) // 검증이 실패했다면 새로운 에러 타입인 WrongPassword를 만들고, 이를 이후에 handle-errors 크레이트에서 처리한다.
                }
            }
            Err(e) => Err(warp::reject::custom(
                handle_errors::Error::ArgonLibraryError(e), // 라이브러리가 실패하면 500 에러를 사용자에게 돌려준다.
            )),
        },
        Err(e) => Err(warp::reject::custom(e)),
    }
}

fn verify_password(hash: &str, password: &[u8]) -> Result<bool, argon2::Error> {
    argon2::verify_encoded(hash, password) // argon2 크레이트는 해시의 일부인 솔트 값을 사용하여 데이터베이스의 해시가 로그인과정에서의 비밀번호와 일치하는지 검증한다.
}

fn issue_token(account_id: AccountId) -> String {
    let current_date_time = Utc::now();
    let dt = current_date_time + chrono::Duration::days(1);

    paseto::tokens::PasetoBuilder::new()
        .set_encryption_key(&Vec::from("RANDOM WORDS WINTER MACINTOSH PC".as_bytes()))
        .set_expiration(&dt)
        .set_not_before(&Utc::now())
        .set_claim("account_id", serde_json::json!(account_id))
        .build()
        .expect("Failed to construct paseto token w/ builder")
}

pub fn auth() -> impl Filter<Extract = (Session,), Error = warp::Rejection> + Clone {
    warp::header::<String>("Authorization").and_then(|token: String| {
        let token = match verify_token(token) {
            Ok(t) => t,
            Err(_) => return future::ready(Err(warp::reject::reject())),
        };

        future::ready(Ok(token))
    })
}
