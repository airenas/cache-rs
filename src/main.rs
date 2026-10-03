use axum::extract::DefaultBodyLimit;
use axum::middleware;
use cache_rs::{
    handlers::{self, data::Service},
    metrics::Metrics,
};
use clap::Parser;
use moka::future::Cache;
use moka::policy::EvictionPolicy;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::time;
use tokio_util::sync::CancellationToken;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use tokio::signal::unix::{signal, SignalKind};

use axum::{
    routing::{delete, get, post},
    Router,
};

/// ML POS tagger http service
#[derive(Parser, Debug)]
#[command(version = env!("CARGO_APP_VERSION"), name = "cache-rs", about="Cache service", 
    long_about = None, author="Airenas V.<airenass@gmail.com>")]
struct Args {
    /// Server port
    #[arg(long, env, default_value = "8000")]
    port: u16,
    /// Max items for cache
    #[arg(long, env = "CACHE_ITEMS", required = false, default_value = "100000")]
    cache_items: u64,
    /// cache time to idle
    #[arg(short, long, env = "CACHE_TIME_TO_IDLE", default_value = "6h", 
        value_parser = |s: &str| s.parse::<humantime::Duration>().map(Into::<Duration>::into) )]
    cache_time_to_idle: Duration,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    // console_subscriber::init();
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::Layer::default().compact())
        .init();
    let args = Args::parse();
    if let Err(e) = main_int(args).await {
        tracing::error!("{}", e);
        return Err(e);
    }
    Ok(())
}

async fn main_int(cfg: Args) -> anyhow::Result<()> {
    tracing::info!("Starting Cache service");
    tracing::info!(version = env!("CARGO_APP_VERSION"));
    tracing::info!(port = cfg.port);
    tracing::info!(cache_items = cfg.cache_items);
    tracing::info!(cache_time_to_idle = ?cfg.cache_time_to_idle);

    let cancel_token = CancellationToken::new();

    let ct = cancel_token.clone();

    tokio::spawn(async move {
        let mut int_stream = signal(SignalKind::interrupt()).unwrap();
        let mut term_stream = signal(SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = int_stream.recv() => tracing::info!("Exit event int"),
            _ = term_stream.recv() => tracing::info!("Exit event term"),
        }
        tracing::debug!("sending exit event");
        ct.cancel();
        tracing::debug!("expected drop tx_close");
    });

    let cache: Cache<String, String> = Cache::builder()
        .max_capacity(cfg.cache_items)
        .eviction_policy(EvictionPolicy::tiny_lfu())
        .time_to_idle(Duration::from_secs(60 * 60 * 5)) // 5h
        .build();

    let metrics = Metrics::new()?;

    let srv = Arc::new(RwLock::new(Service {
        calls: 0,
        cache: cache.clone(),
        drop_word_metric: metrics.drop_word_metric(),
    }));

    let metrics_cl = metrics.clone();

    let helper_router = axum::Router::new().route("/live", get(handlers::live::handler));

    let main_router = Router::new()
        .route("/put", post(handlers::put::handler))
        .route("/get", post(handlers::get::handler))
        .route("/cache", delete(handlers::drop::handler))
        .with_state(srv.clone())
        .layer(middleware::from_fn(move |req, next| {
            let mc = metrics_cl.clone();
            async move { mc.observe(req, next).await }
        }));

    let app = Router::new()
        .merge(helper_router)
        .merge(main_router)
        .route("/metrics", get(handlers::metrics::handler))
        .layer((
            DefaultBodyLimit::max(1024 * 1024),
            TraceLayer::new_for_http(),
            TimeoutLayer::with_status_code(
                http::StatusCode::REQUEST_TIMEOUT,
                Duration::from_secs(10),
            ),
        ));

    let ct = cancel_token.clone();
    let timer = tokio::task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    tracing::trace!("check timer");
                    metrics.observe_cache("cache_items", cache.entry_count());
                },
                _ = ct.cancelled() => {
                    break
                }
            }
        }
        tracing::info!("finished cache check timer");
    });

    let listener = TcpListener::bind(format!("0.0.0.0:{}", cfg.port)).await?;

    let handle = axum_server::Handle::<std::net::SocketAddr>::new();
    let shutdown_future = shutdown_signal_handle(handle.clone(), cancel_token.clone());
    tokio::spawn(shutdown_future);

    // Run the server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            cancel_token.cancelled().await;
        })
        .await?;

    tracing::info!("service ended");
    timer.await?;
    tracing::debug!("metrics timer stopped");

    tracing::info!("Bye");
    Ok(())
}

async fn shutdown_signal_handle<A: axum_server::Address>(
    handle: axum_server::Handle<A>,
    cancel_token: CancellationToken,
) {
    cancel_token.cancelled().await;
    tracing::trace!("Received termination signal shutting down");
    handle.graceful_shutdown(Some(Duration::from_secs(10)));
}
