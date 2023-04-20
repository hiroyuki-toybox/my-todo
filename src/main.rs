mod handlers;
mod repositories;

use axum::routing::{get, post};
use axum::{extract::Extension, Router};
use handlers::{all_todo, create_todo, delete_todo, find_todo, update_todo};
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
        .route("/todos", post(create_todo::<T>).get(all_todo::<T>))
        .route(
            "/todos/:id",
            get(find_todo::<T>)
                .delete(delete_todo::<T>)
                .patch(update_todo::<T>),
        )
        // axumアプリケーション内でrepositoryを共有する
        .layer(Extension(Arc::new(repository)))
}

async fn root() -> &'static str {
    "Hello, world!!"
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::repositories::{CreateTodo, Todo};
    use axum::response::Response;
    use axum::{body::Body, http::Request};

    use hyper::{header, Method, StatusCode};
    use tower::ServiceExt;

    fn build_todo_req_with_json(path: &str, method: Method, json_body: String) -> Request<Body> {
        Request::builder()
            .uri(path)
            .method(method)
            .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(json_body))
            .unwrap()
    }

    fn build_todo_req_with_empty(path: &str, method: Method) -> Request<Body> {
        Request::builder()
            .uri(path)
            .method(method)
            .body(Body::empty())
            .unwrap()
    }

    async fn res_to_string(res: Response) -> String {
        let b = res.into_body();
        let bytes = hyper::body::to_bytes(b).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    async fn res_to_todo(res: Response) -> Todo {
        let body = res_to_string(res).await;
        let todo: Todo = serde_json::from_str(&body).expect(&format!("body: {}", body));
        todo
    }

    #[tokio::test]
    async fn should_created_todo() {
        let expected = Todo::new(1, "should_created_todo".to_string());

        let repository = TodoRepositoryForMemory::new();
        let req = build_todo_req_with_json(
            "/todos",
            Method::POST,
            r#"{"text": "should_created_todo" }"#.to_string(),
        );

        let res = create_app(repository).oneshot(req).await.unwrap();
        let todo = res_to_todo(res).await;
        assert_eq!(expected, todo);
    }

    #[tokio::test]
    async fn should_find_todo() {
        // 期待値作成
        let expected = Todo::new(1, "should_find_todo".to_string());
        // repo作成
        let repository = TodoRepositoryForMemory::new();
        // repoから、Todoを作成
        repository.create(CreateTodo::new("should_find_todo".to_string()));
        // リクエストを作成
        let req = build_todo_req_with_empty("/todos/1", Method::GET);
        // レスポンスを作成
        let res = create_app(repository).oneshot(req).await.unwrap();
        // レスポンスから、todoを生成
        let todo = res_to_todo(res).await;
        // expected
        assert_eq!(expected, todo);
    }

    #[tokio::test]
    async fn should_get_all_todos() {
        let expected = Todo::new(1, "should_get_all_todos".to_string());
        let repository = TodoRepositoryForMemory::new();
        repository.create(CreateTodo::new("should_get_all_todos".to_string()));
        let req = build_todo_req_with_empty("/todos", Method::GET);
        let res = create_app(repository).oneshot(req).await.unwrap();
        let body = res_to_string(res).await;
        let todo: Vec<Todo> = serde_json::from_str(&body)
            .expect(&format!("connot convert TOdo instance. boy: {}", body));
        assert_eq!(vec![expected], todo)
    }

    #[tokio::test]
    async fn should_update_todo() {
        let expected = Todo::new(1, "should_update_todo".to_string());
        let repository = TodoRepositoryForMemory::new();
        repository.create(CreateTodo::new("before_should_update_todo".to_string()));

        let req = build_todo_req_with_json(
            "/todos/1",
            Method::PATCH,
            r#"{
              "id": 1,
              "text": "should_update_todo",
              "completed": false
            }"#
            .to_string(),
        );
        let res = create_app(repository).oneshot(req).await.unwrap();
        let todo = res_to_todo(res).await;

        assert_eq!(expected, todo);
    }

    #[tokio::test]
    async fn should_delete_todo() {
        let repository = TodoRepositoryForMemory::new();
        repository.create(CreateTodo::new("should_delete_todo".to_string()));

        let req = build_todo_req_with_empty("/todos/1", Method::DELETE);
        let res = create_app(repository).oneshot(req).await.unwrap();

        assert_eq!(StatusCode::NO_CONTENT, res.status());
    }

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
