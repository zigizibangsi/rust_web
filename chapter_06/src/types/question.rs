use serde::{Deserialize, Serialize};
#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct Question {
    pub id: QuestionId,
    pub title: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize, Debug, Clone, Eq, Hash, Deserialize, PartialEq)]
pub struct QuestionId(pub String);
