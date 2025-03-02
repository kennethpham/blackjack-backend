use mongodb::bson::uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserData {
    pub _id: Uuid,
    pub name: String,
    pub wins: u64,
}

#[derive(Deserialize)]
pub struct PostUserJson {
    pub name: String,
}
