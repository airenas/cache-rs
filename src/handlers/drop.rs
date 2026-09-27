use std::sync::Arc;

use axum::{debug_handler, extract::State};
use moka::future::Cache;
use prometheus::IntCounter;
use tokio::sync::RwLock;

use super::data::{ApiResult, Service};

#[debug_handler]
pub async fn handler(State(service): State<Arc<RwLock<Service>>>) -> ApiResult<String> {
    let (cache, drop_word_metric) = {
        let service = service.read().await;
        (service.cache.clone(), service.drop_word_metric.clone())
    };

    let drop_count = drop_items(cache, drop_word_metric).await;
    Ok(format!("Dropped: {drop_count}"))
}

async fn drop_items(cache: Cache<String, String>, drop_word_metric: IntCounter) -> u64 {
    cache.run_pending_tasks().await;
    let drop_count = cache.entry_count();
    cache.invalidate_all();
    cache.run_pending_tasks().await;
    drop_word_metric.inc_by(drop_count);
    drop_count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn drops_all_entries_and_increments_the_metric() {
        let cache = Cache::new(10);
        cache.insert("one".to_owned(), "1".to_owned()).await;
        cache.insert("two".to_owned(), "2".to_owned()).await;
        let metric = IntCounter::new("test_drop_word_metric", "test").expect("counter");

        let count = drop_items(cache.clone(), metric.clone()).await;

        assert_eq!(count, 2);
        assert_eq!(cache.entry_count(), 0);
        assert_eq!(metric.get(), 2);
    }

    #[tokio::test]
    async fn dropping_an_empty_cache_reports_zero() {
        let cache = Cache::new(10);
        let metric = IntCounter::new("test_empty_drop_word_metric", "test").expect("counter");

        let count = drop_items(cache, metric.clone()).await;

        assert_eq!(count, 0);
        assert_eq!(metric.get(), 0);
    }
}