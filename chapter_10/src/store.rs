// 로컬 JSON 파일을 읽는 부분을 삭제하므로 임포트 세 개는 필요 없다.
use sqlx::Row;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};

use crate::types::{
    account::{Account, AccountId},
    answer::{Answer, AnswerId, NewAnswer},
    question::{NewQuestion, Question, QuestionId},
};

use handle_errors::Error;

#[derive(Debug, Clone)]
pub struct Store {
    pub connection: PgPool, //questions와 answers를 Store의 필드에서 제거하고 연결 풀을 넣는다.
}

impl Store {
    pub async fn new(db_url: &str) -> Self {
        let db_pool = match PgPoolOptions::new()
            .max_connections(5)
            .connect(db_url)
            .await
        {
            Ok(pool) => pool,
            Err(e) => panic!("DB 연결을 하지 못했습니다: {}", e), // 데이터베이스에 연결하지 못하는 경우에는 애플리케이션을 종료하도록 한다.
        };

        Store {
            connection: db_pool,
        }
    }

    pub async fn get_questions(
        &self,
        limit: Option<u32>,
        offset: u32,
    ) -> Result<Vec<Question>, Error> {
        // limit, offset 매개변수를 함수에 전달하여 클라이언트가 페이지 매기기를 원하는지 알려주고 성공했을 때는 질문의 벡터를 반환 받고, 실패했을 때는 에러 타입을 반환 받는다.
        match sqlx::query("SELECT * from questions LIMIT $1 OFFSET $2") // 쿼리 함수를 써서 일반 SQL 문을 작성해 넣었고 쿼리에 전달할 변수에 달러 기호($)와 숫자를 추가한다.
            .bind(limit) // bind 메서드는 SQL 문의 $+숫자 부분을 여기에 지정된 변수로 대체한다.
            .bind(offset) // 두 번째 bind 항목은 offset 변수이다.
            .map(|row: PgRow| Question {
                // 쿼리에서 질문 하나(혹은 전부)를 반환 받고자 하면 map으로 PostgreSQL에서 반환된 row 객체 각각에서 Question을 생성하도록 한다.
                id: QuestionId(row.get("id")),
                title: row.get("title"),
                content: row.get("content"),
                tags: row.get("tags"),
            })
            .fetch_all(&self.connection) // fetch_all 메서드는 SQL 문을 실행하고 추가된 질문 모두를 반환한다.
            .await
        {
            Ok(questions) => Ok(questions),
            Err(error) => {
                tracing::event!(tracing::Level::ERROR, "{:?}", error);
                Err(Error::DatabaseQueryError(error))
            }
        }
    }

    pub async fn add_question(
        &self,
        new_question: NewQuestion,
        account_id: AccountId,
    ) -> Result<Question, Error> {
        match sqlx::query(
            "INSERT INTO questions (title, content, tags)
            VALUES ($1, $2, $3)
            RETURNING id, title, content, tags",
        )
        .bind(new_question.title)
        .bind(new_question.content)
        .bind(new_question.tags)
        .map(|row: PgRow| Question {
            id: QuestionId(row.get("id")),
            title: row.get("title"),
            content: row.get("content"),
            tags: row.get("tags"),
        })
        .fetch_one(&self.connection)
        .await
        {
            Ok(question) => Ok(question),
            Err(error) => {
                tracing::event!(tracing::Level::ERROR, "{:?}", error);
                Err(Error::DatabaseQueryError(error))
            }
        }
    }

    pub async fn update_question(
        &self,
        question: Question,
        id: i32,
        account_id: AccountId, // 경로 핸들러에서 전달된 AccountID 매개변수를 함수에 추가한다.
    ) -> Result<Question, Error> {
        match sqlx::query(
            // 질문을 수정하려는 계쩡이 해당 질문을 소유하는지 확인하는 WHERE 절을 추가한다.
            "UPDATE questions
            SET title = $1, content = $2, tags = $3
            WHERE id = $4 and account_id = $5 
            RETURNING id, title, content, tags",
        )
        .bind(question.title)
        .bind(question.content)
        .bind(question.tags)
        .bind(id)
        .bind(account_id.0) // AccountId에 .0으로 단일 필드 값을 바인딩한다.
        .map(|row: PgRow| Question {
            id: QuestionId(row.get("id")),
            title: row.get("title"),
            content: row.get("content"),
            tags: row.get("tags"),
        })
        .fetch_one(&self.connection)
        .await
        {
            Ok(question) => Ok(question),
            Err(error) => {
                tracing::event!(tracing::Level::ERROR, "{:?}", error);
                Err(Error::DatabaseQueryError(error))
            }
        }
    }

    pub async fn delete_question(
        &self,
        question_id: i32,
        account_id: AccountId,
    ) -> Result<bool, Error> {
        match sqlx::query("DELETE FROM questions WHERE id = $1 AND account_id = $2")
            .bind(question_id)
            .bind(account_id.0)
            .execute(&self.connection)
            .await
        {
            Ok(_) => Ok(true),
            Err(error) => {
                tracing::event!(tracing::Level::ERROR, "{:?}", error);
                Err(Error::DatabaseQueryError(error))
            }
        }
    }

    pub async fn add_answer(
        &self,
        new_answer: NewAnswer,
        account_id: AccountId,
    ) -> Result<Answer, Error> {
        match sqlx::query(
            "INSERT INTO answers (content, question_id, account_id)
        VALUES ($1, $2, $3)
        ",
        )
        .bind(new_answer.content)
        .bind(new_answer.question_id.0)
        .bind(account_id.0)
        .map(|row: PgRow| Answer {
            id: AnswerId(row.get("id")),
            content: row.get("content"),
            question_id: QuestionId(row.get("corresponding_question")),
        })
        .fetch_one(&self.connection)
        .await
        {
            Ok(answer) => Ok(answer),
            Err(error) => {
                tracing::event!(tracing::Level::ERROR, "{:?}", error);
                Err(Error::DatabaseQueryError(error))
            }
        }
    }

    pub async fn add_account(&self, account: Account) -> Result<bool, Error> {
        match sqlx::query("INSERT INTO accounts (email, password) VALUES ($1, $2)")
            .bind(account.email)
            .bind(account.password)
            .execute(&self.connection)
            .await
        {
            Ok(_) => Ok(true),
            Err(error) => {
                tracing::event!(
                    tracing::Level::ERROR,
                    code = error
                        .as_database_error()
                        .unwrap()
                        .code()
                        .unwrap()
                        .parse::<i32>()
                        .unwrap(),
                    db_message = error.as_database_error().unwrap().message(),
                    constraint = error.as_database_error().unwrap().constraint().unwrap()
                );
                Err(Error::DatabaseQueryError(error))
            }
        }
    }

    pub async fn get_account(self, email: String) -> Result<Account, Error> {
        match sqlx::query("SELECT * from accounts where email = $1")
            .bind(email)
            .map(|row: PgRow| Account {
                id: Some(AccountId(row.get("id"))),
                email: row.get("email"),
                password: row.get("password"),
            })
            .fetch_one(&self.connection)
            .await
        {
            Ok(account) => Ok(account),
            Err(error) => {
                tracing::event!(tracing::Level::ERROR, "{:?}", error);
                Err(Error::DatabaseQueryError(error))
            }
        }
    }

    pub async fn is_question_owner(
        &self,
        question_id: i32,
        account_id: &AccountId,
    ) -> Result<bool, Error> {
        match sqlx::query("SELECT * from questions where id = $1 and account_id = $2") // get_questions에서 사용했던 SELECT 쿼리에 id와 account_id를 넣은 WHERE 절을 사용한다.
            .bind(question_id)
            .bind(account_id.0)
            .fetch_optional(&self.connection) // fetch_optional은 None이나 결과 값 하나를 돌려준다.
            .await
        {
            Ok(question) => Ok(question.is_some()), // 결과가 "있는지"를 검사하고, 없다면 false를 반환한다.
            Err(e) => {
                tracing::event!(tracing::Level::ERROR, "{:?}", e);
                Err(Error::DatabaseQueryError(e))
            }
        }
    }
}
