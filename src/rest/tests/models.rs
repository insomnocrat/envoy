use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserList {
    pub data: Vec<UserPreview>,
    pub total: usize,
    pub page: usize,
    pub limit: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserPreview {
    pub id: String,
    pub title: String,
    pub first_name: String,
    pub last_name: String,
    pub picture: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub date_of_birth: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub register_date: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub street: String,
    pub city: String,
    pub state: String,
    pub country: String,
    pub timezone: String,
}
