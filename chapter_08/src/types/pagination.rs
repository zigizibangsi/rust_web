use handle_errors::Error;
use std::collections::HashMap;

/// Pagination 구조체는 쿼리 매개변수에서 추출된다
#[derive(Default, Debug)]
pub struct Pagination {
    /// 반환될 마지막 아이템의 인덱스
    /// Pagination의 첫 번째 필드의 이름을 limit으로 바꾼다. 이 값은 None이나 숫자가 될 수 있다.
    /// None을 전달하면 PostgreSQL은 기본적으로 이를 무시하는데, 덕택에 여분의 if 문을 쓸 필요가 없어진다.
    pub limit: Option<u32>,
    /// 반환될 첫 번째 아이템의 인덱스
    /// 두 번째 매개변수는 offset으로 값이 0이면 PostgreSQL은 이 값을 무시한다.
    /// 제한 필드와 동일하다. 덕분에 if 문을 쓰지 않아도 된다.
    pub offset: u32,
}

/// 매개변수를 /questions 경로에서 추출하기
/// # 예제 쿼리
/// 이 경로에 대한 GET 요청에는 반환 받기 원하는 질문만 반환 받도록
/// 페이지 정보가 추가될 수 있다
/// /questions?start=1&end=10
/// # 사용 예
/// ```rust
/// let mut query = HashMap::new();
/// query.insert("limit".to_string(), "1".to_string());
/// query.insert("offset".to_string(), "10".to_string());
/// let p = types::pagination::extract_pagination(query).unwrap();
/// assert_eq!(p.limit, 1);
/// assert_eq!(p.offset, 10);
/// ```

pub fn extract_pagination(params: HashMap<String, String>) -> Result<Pagination, Error> {
    // 나중에 더 개선할 수 있다
    if params.contains_key("limit") && params.contains_key("offset") {
        return Ok(Pagination {
            // limit 매개변수를 쿼리에서 가져와
            // 숫자로 변환을 시도한다
            limit: Some(
                params
                    .get("limit")
                    .unwrap()
                    .parse::<u32>() // &str을 u32로 변환한다.
                    .map_err(Error::ParseError)?,
            ),
            // offset 매개변수를 쿼리에서 가져와
            // 숫자로 변환하려고 한다
            offset: params
                .get("offset")
                .unwrap()
                .parse::<u32>() // &str을 u32로 변환한다.
                .map_err(Error::ParseError)?,
        });
    }

    Err(Error::MissingParameters)
}
