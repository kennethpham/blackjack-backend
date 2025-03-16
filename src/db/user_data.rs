use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub _id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub wins: i32,
}

#[derive(Deserialize)]
pub struct PostUserJson {
    pub name: String,
}
