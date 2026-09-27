use std::sync::Arc;

use axum::{debug_handler, extract::State, Json};
use moka::future::Cache;
use tokio::sync::RwLock;

use super::data::{ApiResult, Item, Service};
use super::error::ApiError;

const MAX_BATCH_SIZE: usize = 1_000_000;
const MAX_KEY_BYTES: usize = 200;
#[debug_handler]
pub async fn handler(
    State(service): State<Arc<RwLock<Service>>>,
    Json(input): Json<Vec<Item>>,
) -> ApiResult<&'static str> {
    let cache = service.read().await.cache.clone();
    put_items(cache, input).await?;
    Ok("OK")
}

async fn put_items(cache: Cache<String, String>, input: Vec<Item>) -> ApiResult<()> {
    if input.len() > MAX_BATCH_SIZE {
        return Err(ApiError::BadRequest(
            "Invalid request".to_owned(),
            format!("batch size exceeds {MAX_BATCH_SIZE}"),
        ));
    }

    if input
        .iter()
        .any(|item| item.key.is_empty() || item.key.len() > MAX_KEY_BYTES)
    {
        return Err(ApiError::BadRequest(
            "Invalid request".to_owned(),
            format!("keys must contain 1 to {MAX_KEY_BYTES} bytes"),
        ));
    }

    if input.iter().any(|item| item.value.is_none()) {
        return Err(ApiError::BadRequest(
            "Invalid request".to_owned(),
            "each item must include a value".to_owned(),
        ));
    }

    for item in input {
        if let Some(value) = item.value {
            cache.insert(item.key, value).await;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn inserts_values_and_overwrites_existing_entries() {
        let cache = Cache::new(2);
        cache.insert("present".to_owned(), "value".to_owned()).await;

        put_items(
            cache.clone(),
            vec![
                Item {
                    key: "present".to_owned(),
                    value: Some("updated".to_owned()),
                },
                Item {
                    key: "new".to_owned(),
                    value: Some("value".to_owned()),
                },
            ],
        )
        .await
        .expect("valid put request");

        assert_eq!(cache.get("present").await.as_deref(), Some("updated"));
        assert_eq!(cache.get("new").await.as_deref(), Some("value"));
    }

    #[tokio::test]
    async fn rejects_invalid_key_without_mutating_cache() {
        let cache = Cache::new(1);
        let result = put_items(
            cache.clone(),
            vec![Item {
                key: String::new(),
                value: Some("value".to_owned()),
            }],
        )
        .await;

        assert!(matches!(result, Err(ApiError::BadRequest(_, _))));
        assert_eq!(cache.entry_count(), 0);
    }

    #[tokio::test]
    async fn rejects_missing_values_without_mutating_cache() {
        let cache = Cache::new(1);
        let result = put_items(
            cache.clone(),
            vec![
                Item {
                    key: "valid".to_owned(),
                    value: Some("value".to_owned()),
                },
                Item {
                    key: "missing-value".to_owned(),
                    value: None,
                },
            ],
        )
        .await;

        assert!(matches!(result, Err(ApiError::BadRequest(_, _))));
        assert_eq!(cache.entry_count(), 0);
    }

    #[tokio::test]
    async fn rejects_batches_above_the_limit() {
        let cache = Cache::new(1);
        let input = (0..=MAX_BATCH_SIZE)
            .map(|index| Item {
                key: index.to_string(),
                value: Some("value".to_owned()),
            })
            .collect();

        let result = put_items(cache.clone(), input).await;

        assert!(matches!(result, Err(ApiError::BadRequest(_, _))));
        assert_eq!(cache.entry_count(), 0);
    }
}
