use moka::future::Cache;
use prometheus::IntCounter;
use serde::{Deserialize, Serialize};

use super::error::ApiError;

pub struct Service {
    pub cache: Cache<String, String>,
    pub drop_word_metric: IntCounter,
    pub calls: u64,
}

pub type ApiResult<T> = std::result::Result<T, ApiError>;

#[derive(Debug, Serialize, Clone)]
pub struct LiveResponse {
    pub status: bool,
    pub version: String,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct Item {
    pub key: String,
    pub value: Option<String>,
}
