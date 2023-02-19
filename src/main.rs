mod handlers;
mod repositories;

use axum::routing::{get, post};
use axum::{extract::Extension, Router};
use handlers::create_todo;
use repositories::TodoRepository;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::repositories::TodoRepositoryForMemory;

#[tokio::main]
async fn main() {
    // logging
    let log_level = env::var("RUST_LOG").unwrap_or("info".to_string());
    env::set_var("Rust_LOG", log_level);
    tracing_subscriber::fmt::init();

    let repository = TodoRepositoryForMemory::new();
    let app = create_app(repository);
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn create_app<T: TodoRepository>(repository: T) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/todos", post(create_todo::<T>))
        // axumアプリケーション内でrepositoryを共有する
        .layer(Extension(Arc::new(repository)))
}

async fn root() -> &'static str {
    "Hello, world!!"
}

#[cfg(test)]
mod test {
    use super::*;
    use axum::{body::Body, http::Request};

    use tower::ServiceExt;

    #[tokio::test]
    async fn should_return_hello_world() {
        let repository = TodoRepositoryForMemory::new();
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let res = create_app(repository).oneshot(req).await.unwrap();
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(body, "Hello, world!!");
    }
}
