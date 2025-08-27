use serde::{Deserialize, Serialize};

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

pub async fn check_profanity(content: String) -> Result<String, handle_errors::Error> {
    let client = reqwest::Client::new(); // HTTP 요청을 보내는 새 클라이언트를 생성한다.
    let res = client
        .post("https://api.apilayer.com/bad_words?censor_character=*") // post 메서드는 HTTP POST를 보내며 URL로 &str을 받는다.
        .header("apikey", "xxxxxxxx") // 키-값 쌍으로 인증 헤더 값을 수동으로 추가한다.
        .body("a list with shit words") // 본문에는 금칙 단어를 검사할 내용을 담는다.
        .send()
        .await // send 메서드는 비동기이며 에러를 반환할 수 있으므로 .await와 ?를 뒤에 붙인다.
        .map_err(|e| handle_errors::Error::MiddlewareReqwestAPIError(e))? 

    if !res.status().is_success() { // 응답 상태가 성공인지 검사한다.
        if res.status().is_client_error() { // 상태 값은 클라이언트 에러인지 서버 에러인지도 알려준다.
            let err = transform_error(res).await; // APILayer API의 에러 메시지가 썩 좋지 않으니 자체적인 메시지를 만든다.
            return Err(warp::reject::custom(handle_errors::Error::ClientError(err))); // APILayerError에 캡슐화한 클라이언트 에러나 서버 에러를 반환한다.
        } else {
            let err = transform_error(res).await;
            return Err(warp::reject::custom(handle_errors::Error::ServerError(err)));
        }
    }

    match res.json::<BadWordsResponse>().await {
        Ok(res) => Ok(res.censored_content),
        Err(e) => Err(handle_errors::Error::ReqwestAPIError(e)),
    }
}

async fn transform_error(res: reqwest::Response) -> handle_errors::APILayerError { // 응답 값을 받아 (이 시점에서는 우리는 해당 값이 에러임을 안다) 해당 메시지에 상태 코드 값을 추가한다.
    handle_errors::APILayerError {
        status: res.status().as_u16(),
        message: res.json::<APIResponse>().await.unwrap().message,
    }
}