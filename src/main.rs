use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

struct AppState {
    todos: Vec<Todo>,
}

type Db = Arc<Mutex<AppState>>;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Todo {
    id: Uuid,
    title: String,
    completed: bool,
    due: Option<DateTime<Utc>>,
}

impl Todo {
    fn new(title: impl Into<String>, due: Option<DateTime<Utc>>) -> Self {
        Todo {
            id: Uuid::new_v4(),
            title: title.into(),
            completed: false,
            due,
        }
    }
}

async fn root() -> &'static str {
    "Hello"
}

async fn get_todos(State(state): State<Db>) -> Json<Value> {
    Json(json!({"todos": state.lock().unwrap().todos.clone()}))
}

async fn get_todo(
    Path(id): Path<String>,
    State(state): State<Db>,
) -> Result<impl IntoResponse, StatusCode> {
    if let Some(todo) = state
        .lock()
        .unwrap()
        .todos
        .iter()
        .find(|t| t.id.to_string() == id)
    {
        Ok((StatusCode::OK, Json(todo.clone())))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn delete_todo(
    Path(id): Path<String>,
    State(state): State<Db>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut db = state.lock().unwrap();
    if let Some(index) = db.todos.iter().position(|t| t.id.to_string() == id) {
        db.todos.remove(index);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[tokio::main]
async fn main() {
    let db = Connection::open("./db.sqlite").unwrap();
    let shared_state = Arc::new(Mutex::new(AppState { todos: vec![] }));
    let app = Router::new()
        .route("/", get(root))
        .route("/todos", get(get_todos))
        .route("/todos/:id", get(get_todo).delete(delete_todo))
        .with_state(shared_state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
