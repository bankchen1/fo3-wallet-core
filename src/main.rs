//! 最小化钱包 API

use std::net::SocketAddr;

use axum::{
    routing::get,
    Router,
};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 构建应用
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(TraceLayer::new_for_http());

    // 运行服务器
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("监听端口 {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
