use std::collections::HashMap;

// 런타임 사용은 애플리케이션의 main 함수 위에 정의한다.
#[tokio::main]
// main 함수를 async로 표시하므로 내부에서 await를 사용할 수 있다.
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 여기에서 Reqwest 크레이트를 사용해 Future 타입을 반환하는 HTTP GET 요청을 실행한다.
    let resp = reqwest::get("https://httpbin.org/ip")
        .await? // await 키워드를 사용해 이 함수에서 다음으로 넘어가기 전에 퓨처가 완료될 때까지 기다릴 것이라고 프로그램에 알린다.
        .json::<HashMap<String, String>>()
        .await?;
    println!("{:#?}", resp); // 응답 내용을 출력한다.
    Ok(()) // OK 키워드는 Result를 반환하며, 이 경우에서는 빈 값이다.
}
