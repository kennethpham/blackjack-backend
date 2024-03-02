use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserData {
    pub name: String,
    pub wins: u64,
}

#[derive(Deserialize)]
pub struct PostUserJson {
    pub name: String,
}
