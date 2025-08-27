use serde_json::json; // serde_json에서 json! 매크로를 임포트한다.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new(); // HTTP 요청을 보내는 새 클라이언트를 생성한다.
    let res = client
        .post("https://api.apilayer.com/bad_words?censor_character=*") // post 메서드는 HTTP POST를 보내며 URL로 &str을 받는다.
        .header("apikey", "xxxxxxxx") // 키-값 쌍으로 인증 헤더 값을 수동으로 추가한다.
        .body("a list with shit words") // 본문에는 금칙 단어를 검사할 내용을 담는다.
        .send()
        .await? // send 메서드는 비동기이며 에러를 반환할 수 있으므로 .await와 ?를 뒤에 붙인다.

    let status_code = res.status();
    let message = res.text().await?;

    let response = json!({ // 응답을 JSON으로 바꾸는 데 json! 매크로를 사용한다.
        "StatusCode": status_code.as_str(),
        "Message": message,
    });

    println!("{:#?}", response);

    Ok(())
}
