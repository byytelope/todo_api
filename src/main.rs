use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

struct AppState {
    db: Connection,
}

type Db = Arc<Mutex<AppState>>;

#[derive(Serialize)]
struct Response;

impl Response {
    fn todo(todo: Todo) -> Json<Value> {
        Json(json!({"todo": todo}))
    }

    fn todos(todos: Vec<Todo>) -> Json<Value> {
        Json(json!({"todos": todos}))
    }

    fn empty() -> Json<Value> {
        Json(json!({}))
    }
}

type HttpResponse = axum::response::Result<(StatusCode, Json<Value>), StatusCode>;

#[derive(Deserialize)]
struct TodoPartial {
    title: String,
    due: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Todo {
    id: Uuid,
    title: String,
    completed: bool,
    due: Option<DateTime<Utc>>,
}

impl From<TodoPartial> for Todo {
    fn from(partial: TodoPartial) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: partial.title,
            completed: false,
            due: partial.due,
        }
    }
}

impl From<&Row<'_>> for Todo {
    fn from(row: &Row<'_>) -> Self {
        Self {
            id: row.get(0).unwrap(),
            title: row.get(1).unwrap(),
            completed: row.get(2).unwrap(),
            due: row.get(3).unwrap(),
        }
    }
}

async fn root() -> &'static str {
    "Hello"
}

async fn add_todo(State(state): State<Db>, Json(todo_partial): Json<TodoPartial>) -> HttpResponse {
    let todo = Todo::from(todo_partial);
    let st = state.lock().unwrap();
    let res = st.db.execute(
        "INSERT INTO todos (id, title, completed, due) VALUES (?1, ?2, ?3, ?4)",
        (todo.id, todo.title, todo.completed, todo.due),
    );

    match res {
        Ok(num_rows) => {
            if num_rows == 1 {
                Ok((StatusCode::CREATED, Response::empty()))
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_todos(State(state): State<Db>) -> HttpResponse {
    let st = state.lock().unwrap();
    let mut query = st.db.prepare("SELECT * FROM todos").unwrap();
    let todos = query
        .query_map([], |todo| Ok(Todo::from(todo)))
        .unwrap()
        .map(|todo| todo.unwrap())
        .collect::<Vec<Todo>>();

    Ok((StatusCode::OK, Response::todos(todos)))
}

async fn get_todo(State(state): State<Db>, Path(id): Path<Uuid>) -> HttpResponse {
    let st = state.lock().unwrap();
    let res = st
        .db
        .query_row("SELECT * FROM todos WHERE id = ?1", [id], |row| {
            Ok(Todo::from(row))
        });

    match res {
        Ok(todo) => Ok((StatusCode::OK, Response::todo(todo))),
        Err(e) => {
            eprintln!("{}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

async fn delete_todo(State(state): State<Db>, Path(id): Path<Uuid>) -> HttpResponse {
    let st = state.lock().unwrap();
    let res = st.db.execute("DELETE FROM todos WHERE id = ?", [id]);

    match res {
        Ok(num_rows) => {
            if num_rows == 0 {
                Err(StatusCode::NOT_FOUND)
            } else {
                Ok((StatusCode::NO_CONTENT, Response::empty()))
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tokio::main]
async fn main() {
    let db = Connection::open("./db.sqlite").unwrap();
    db.execute(
        "CREATE TABLE IF NOT EXISTS todos (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            completed INTEGER NOT NULL CHECK (completed IN (0, 1)),
            due TEXT
        )",
        (),
    )
    .unwrap();
    let shared_state = Arc::new(Mutex::new(AppState { db }));
    let app = Router::new()
        .route("/", get(root))
        .route("/todos/:id", get(get_todo).delete(delete_todo))
        .route("/todos", get(get_todos).post(add_todo))
        .with_state(shared_state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
