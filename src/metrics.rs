use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use prometheus::HistogramOpts;
use prometheus::HistogramVec;
use prometheus::IntCounter;
use prometheus::IntGaugeVec;
use prometheus::Opts;

#[derive(Debug, Clone)]
pub struct Metrics {
    http_perf: HistogramVec,
    cache_sizes: IntGaugeVec,
    drop_word_metric: IntCounter,
}

impl Metrics {
    pub fn new() -> anyhow::Result<Self> {
        let http_perf = HistogramVec::new(
            HistogramOpts::new(
                "http_response_time_seconds",
                "HTTP method response time in seconds.",
            ),
            &["path", "status"],
        )?;
        let cache_sizes = IntGaugeVec::new(
            Opts::new("cache_items", "Items count in the cache."),
            &["cache"],
        )?;
        let drop_word_metric = IntCounter::new(
            "drop_word_metric",
            "Number of cache words dropped through the drop endpoint.",
        )?;

        prometheus::default_registry().register(Box::new(http_perf.clone()))?;
        prometheus::default_registry().register(Box::new(cache_sizes.clone()))?;
        prometheus::default_registry().register(Box::new(drop_word_metric.clone()))?;

        Ok(Self {
            http_perf,
            cache_sizes,
            drop_word_metric,
        })
    }

    pub fn drop_word_metric(&self) -> IntCounter {
        self.drop_word_metric.clone()
    }

    pub fn observe_cache(&self, name: &str, count: u64) {
        self.cache_sizes
            .with_label_values(&[name])
            .set(count as i64);
    }

    pub async fn observe(&self, request: Request, next: Next) -> Response {
        let path = request.uri().path().to_string();
        let start_time = Instant::now();

        let response = next.run(request).await;
        let status_code = response.status();

        self.http_perf
            .with_label_values(&[&path, status_code.as_str()])
            .observe(start_time.elapsed().as_secs_f64());
        response
    }
}
