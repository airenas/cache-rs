use std::sync::Arc;

use axum::{
    debug_handler,
    extract::{self, State},
    Json,
};
use futures::stream::{self, StreamExt};
use moka::future::Cache;
use tokio::sync::RwLock;

use super::data::{ApiResult, Item, Service};
use super::error::ApiError;

const MAX_BATCH_SIZE: usize = 1_000_000;
const MAX_KEY_BYTES: usize = 200;
const LOOKUP_CONCURRENCY: usize = 64;

#[debug_handler]
pub async fn handler(
    State(service): State<Arc<RwLock<Service>>>,
    Json(input): Json<Vec<Item>>,
) -> ApiResult<extract::Json<Vec<Item>>> {
    let cache = service.read().await.cache.clone();
    let result = lookup_items(cache, input).await?;
    Ok(Json(result))
}

async fn lookup_items(cache: Cache<String, String>, input: Vec<Item>) -> ApiResult<Vec<Item>> {
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

    Ok(stream::iter(input)
        .map(|item| {
            let cache = cache.clone();
            async move {
                let value = cache.get(&item.key).await;
                Item {
                    key: item.key,
                    value,
                }
            }
        })
        .buffered(LOOKUP_CONCURRENCY)
        .collect()
        .await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn looks_up_values_preserves_order_and_returns_misses() {
        let cache = Cache::new(2);
        cache.insert("present".to_owned(), "value".to_owned()).await;

        let result = lookup_items(
            cache,
            vec![
                Item {
                    key: "missing".to_owned(),
                    value: Some("ignored".to_owned()),
                },
                Item {
                    key: "present".to_owned(),
                    value: None,
                },
            ],
        )
        .await
        .expect("valid lookup request");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, "missing");
        assert_eq!(result[0].value, None);
        assert_eq!(result[1].key, "present");
        assert_eq!(result[1].value.as_deref(), Some("value"));
    }

    #[tokio::test]
    async fn rejects_empty_or_oversized_keys() {
        let cache = Cache::new(1);
        let result = lookup_items(
            cache,
            vec![Item {
                key: String::new(),
                value: None,
            }],
        )
        .await;

        assert!(matches!(result, Err(ApiError::BadRequest(_, _))));
    }

    #[tokio::test]
    async fn rejects_batches_above_the_limit() {
        let cache = Cache::new(1);
        let input = (0..=MAX_BATCH_SIZE)
            .map(|index| Item {
                key: index.to_string(),
                value: None,
            })
            .collect();

        let result = lookup_items(cache, input).await;

        assert!(matches!(result, Err(ApiError::BadRequest(_, _))));
    }
}
