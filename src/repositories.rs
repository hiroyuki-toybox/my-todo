use std::{
    collections::HashMap,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use anyhow::Context;
use axum::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use thiserror::Error;
use validator::Validate;

#[derive(Debug, Error)]
enum RepositoryError {
    #[error("NotFound, id is {0}")]
    NotFound(i32),
}

#[async_trait]
pub trait TodoRepository: Clone + std::marker::Send + std::marker::Sync + 'static {
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo>;
    async fn find(&self, id: i32) -> anyhow::Result<Todo>;
    async fn all(&self) -> anyhow::Result<Vec<Todo>>;
    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo>;
    async fn delete(&self, id: i32) -> anyhow::Result<()>;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Todo {
    id: i32,
    text: String,
    completed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Validate)]
pub struct CreateTodo {
    #[validate(length(min = 1, message = "can not be empty"))]
    #[validate(length(max = 100, message = "can not be over 100"))]
    text: String,
}

#[cfg(test)]
impl CreateTodo {
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Validate)]
pub struct UpdateTodo {
    #[validate(length(min = 1, message = "can not be empty"))]
    #[validate(length(max = 100, message = "can not be over 100"))]
    text: Option<String>,
    completed: Option<bool>,
}

impl Todo {
    pub fn new(id: i32, text: String) -> Self {
        Self {
            id,
            text,
            completed: false,
        }
    }
}

type TodoDatas = HashMap<i32, Todo>;

#[derive(Debug, Clone)]
pub struct TodoRepositoryForMemory {
    store: Arc<RwLock<TodoDatas>>,
}

impl TodoRepositoryForMemory {
    pub fn new() -> Self {
        TodoRepositoryForMemory {
            store: Arc::default(),
        }
    }

    fn write_store_ref(&self) -> RwLockWriteGuard<TodoDatas> {
        self.store.write().unwrap()
    }

    fn read_store_ref(&self) -> RwLockReadGuard<TodoDatas> {
        self.store.read().unwrap()
    }
}

#[async_trait]
impl TodoRepository for TodoRepositoryForMemory {
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo> {
        let mut store = self.write_store_ref();
        let id = (store.len() + 1) as i32;
        let todo = Todo::new(id, payload.text.clone());
        store.insert(id, todo.clone());
        Ok(todo)
    }
    async fn find(&self, id: i32) -> anyhow::Result<Todo> {
        let store = self.read_store_ref();
        let todo = store
            .get(&id)
            .map(|todo| todo.clone())
            .ok_or(RepositoryError::NotFound(id))?;

        Ok(todo)
    }
    async fn all(&self) -> anyhow::Result<Vec<Todo>> {
        let store = self.read_store_ref();
        Ok(Vec::from_iter(store.values().map(|todo| todo.clone())))
    }
    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo> {
        let mut store = self.write_store_ref();
        let todo = store.get(&id).context(RepositoryError::NotFound(id))?;
        let text = payload.text.unwrap_or(todo.text.clone());
        let completed = payload.completed.unwrap_or(todo.completed);

        let todo = Todo {
            id,
            text,
            completed,
        };
        store.insert(id, todo.clone());
        Ok(todo)
    }
    async fn delete(&self, id: i32) -> anyhow::Result<()> {
        let mut store = self.write_store_ref();
        store.remove(&id).ok_or(RepositoryError::NotFound(id))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TodoRepositoryForDb {
    pool: PgPool,
}

impl TodoRepositoryForDb {
    pub fn new(pool: PgPool) -> Self {
        TodoRepositoryForDb { pool }
    }
}

#[async_trait]
impl TodoRepository for TodoRepositoryForDb {
    async fn create(&self, _payload: CreateTodo) -> anyhow::Result<Todo> {
        todo!()
    }
    async fn find(&self, id: i32) -> anyhow::Result<Todo> {
        todo!()
    }
    async fn all(&self) -> anyhow::Result<Vec<Todo>> {
        todo!()
    }
    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo> {
        todo!()
    }
    async fn delete(&self, id: i32) -> anyhow::Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn todo_curd_scenario() {
        let text = "todo text".to_string();
        let id = 1;
        let expected = Todo::new(id, text.clone());

        // create
        let repository = TodoRepositoryForMemory::new();
        let todo = repository
            .create(CreateTodo { text: text.clone() })
            .await
            .expect("failed");
        assert_eq!(todo, expected);

        // find
        let todo = repository.find(id).await.unwrap();
        assert_eq!(todo, expected);

        // all
        let todos = repository.all().await.unwrap();
        assert_eq!(todos, vec![expected.clone()]);

        // update
        let text = "update todo".to_string();
        let todo = repository
            .update(
                id,
                UpdateTodo {
                    text: Some(text.clone()),
                    completed: None,
                },
            )
            .await
            .unwrap();

        let expected = Todo {
            id,
            text,
            completed: false,
        };

        assert_eq!(todo, expected);

        // delete
        repository.delete(id).await.unwrap();
        let todo = repository.find(id).await;
        assert!(!todo.is_ok());
    }
}
