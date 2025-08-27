use argon2::Error as ArgonError;
use warp::{
    Rejection, Reply,
    filters::{body::BodyDeserializeError, cors::CorsForbidden},
    http::StatusCode,
    reject::Reject,
};

use reqwest::Error as ReqwestError;
use reqwest_middleware::Error as MiddlewareReqwestError;

use tracing::{Level, event, instrument};

#[derive(Debug)]
pub enum Error {
    ParseError(std::num::ParseIntError),
    MissingParameters,
    WrongPassword,
    CannotDecryptToken,
    Unauthorized,
    ArgonLibraryError(ArgonError),
    DatabaseQueryError(sqlx::Error), // DatabaseQueryError에 점검해야 할 sqlx::Error를 매개변수로 추가한다.
    ReqwestAPIError(ReqwestError),
    MiddlewareReqwestAPIError(MiddlewareReqwestError),
    ClientError(APILayerError), // HTTP 클라이언트(Reqwest) 에서 에러가 발생할 경우를 위해 ClientError 열거 값을 만든다.
    ServerError(APILayerError), // 외부 API에서 4xx이나 5xx HTTP 상태 코드를 반환하는 경우를 위해 ServerError 열거 값을 만든다.
}

#[derive(Debug, Clone)]
pub struct APILayerError {
    // 해당 에러 값 중 일부를 뽑아 도우미 함수(helper function)를 이용하여 새로운 Error 타입으로 반환할 수 있도록 재구성한다.
    pub status: u16,
    pub message: String,
}

impl std::fmt::Display for APILayerError {
    // 로깅을 하거나 직접 에러를 출력할 것이므로 Display 트레이트를 직접 구현한다.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Status: {}, Message: {}", self.status, self.message)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            Error::ParseError(err) => {
                write!(f, "Cannot parse parameter: {}", err)
            }
            Error::MissingParameters => {
                write!(f, "Missing parameters")
            }
            Error::WrongPassword => {
                write!(f, "Wrong password")
            }
            Error::CannotDecryptToken => {
                write!(f, "Cannot decrypt error")
            }
            Error::Unauthorized => {
                write!(f, "No permission to change the underlying resource")
            }
            Error::ArgonLibraryError(_) => {
                write!(f, "Cannot verify password")
            }
            Error::DatabaseQueryError(_) => {
                write!(f, "Cannot update, invalid data.") // 에러를 출력하려 할 때 (아직은) 실제 에러 값을 신경 쓰지 않는다.
            }
            Error::ReqwestAPIError(err) => {
                write!(f, "External API error: {}", err)
            }
            Error::MiddlewareReqwestAPIError(err) => {
                write!(f, "External API error: {}", err)
            }
            Error::ClientError(err) => {
                write!(f, "External Client error: {}", err)
            }
            Error::ServerError(err) => {
                write!(f, "External Server error: {}", err)
            }
        }
    }
}

impl Reject for Error {}
impl Reject for APILayerError {}

const DUPLICATE_KEY: u32 = 23505;

#[instrument]
pub async fn return_error(r: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(crate::Error::DatabaseQueryError(e)) = r.find() {
        // 다음 코드 블록에서 사용할 수 있도록 if 절에 매개변수를 추가한다.
        event!(Level::ERROR, "Database query error");
        match e {
            // 데이터베이스 에러를 처리하기 위해 sqlx::Error 패턴을 검사한다.
            sqlx::Error::Database(err) => {
                if err.code().unwrap().parse::<u32>().unwrap() == 
                // 데이터베이스 에리어니 경우 코드 필드가 있음을 알고 있다. &str 결과 값을 u32로 파싱하여 찾는 값인지 비교한다.
                DUPLICATE_KEY {
                    Ok(warp::reply::with_status(
                        "Account already exists".to_string(), // 우리가 찾는 코드가 맞다면, 계정이 이미 존재한다는 메시지를 반환한다.
                        StatusCode::UNPROCESSABLE_ENTITY,
                    ))
                } else {
                    Ok(warp::reply::with_status(
                        "Cannot update data".to_string(),
                        StatusCode::UNPROCESSABLE_ENTITY,
                    ))
                }
            }
            _ => Ok(warp::reply::with_status(
                "Cannot update data".to_string(),
                StatusCode::UNPROCESSABLE_ENTITY,
            )),
        }
    } else if let Some(crate::Error::ReqwestAPIError(e)) = r.find() {
        // 새로운 에러를 확인하고, 에러를 발견하면 세부 정보를 기록하고 클라이언트에게 500을 반환하는 if/else 블록을 확장한다.
        event!(Level::ERROR, "{}", e);
        Ok(warp::reply::with_status(
            "Internal Server Error".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    } else if let Some(crate::Error::Unauthorized) = r.find() {
        event!(Level::ERROR, "Not matching account id");
        Ok(warp::reply::with_status(
            "No permission to change underlying resource".to_string(),
            StatusCode::UNAUTHORIZED,
        ))
    } else if let Some(crate::Error::WrongPassword) = r.find() {
        event!(Level::ERROR, "Entered wrong password");
        Ok(warp::reply::with_status(
            "Wrong E-Mail/Password combination".to_string(),
            StatusCode::UNAUTHORIZED,
        ))
    } else if let Some(crate::Error::MiddlewareReqwestAPIError(e)) = r.find() {
        event!(Level::ERROR, "{}", e);
        Ok(warp::reply::with_status(
            "Internal Server Error".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    } else if let Some(crate::Error::ClientError(e)) = r.find() {
        event!(Level::ERROR, "{}", e);
        Ok(warp::reply::with_status(
            "Internal Server Error".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    } else if let Some(crate::Error::ServerError(e)) = r.find() {
        event!(Level::ERROR, "{}", e);
        Ok(warp::reply::with_status(
            "Internal Server Error".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    } else if let Some(error) = r.find::<CorsForbidden>() {
        event!(Level::ERROR, "CORS forbidden error: {}", error);
        Ok(warp::reply::with_status(
            error.to_string(),
            StatusCode::FORBIDDEN,
        ))
    } else if let Some(error) = r.find::<BodyDeserializeError>() {
        event!(Level::ERROR, "Cannot deserialize request body: {}", error);
        Ok(warp::reply::with_status(
            error.to_string(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ))
    } else if let Some(error) = r.find::<Error>() {
        event!(Level::ERROR, "{}", error);
        Ok(warp::reply::with_status(
            error.to_string(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ))
    } else {
        event!(Level::WARN, "Requested route was not found");
        Ok(warp::reply::with_status(
            "Route not found".to_string(),
            StatusCode::NOT_FOUND,
        ))
    }
}
