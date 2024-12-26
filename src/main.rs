use axum::{
    routing::{get, post, delete, put},
    Router,
    Json,
    extract::Path,
    serve,
};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, FromRow};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::fs;
use tower_http::cors::{CorsLayer, Any};

#[derive(Debug, Serialize, FromRow)]
struct Todo {
    id: i64,
    title: String,
    completed: bool,
}

#[derive(Debug, Deserialize)]
struct CreateTodo {
    title: String,
}

#[derive(Debug, Deserialize)]
struct UpdateTodo {
    title: Option<String>,
    completed: Option<bool>,
}

struct AppState {
    db: SqlitePool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_dir = "./";
    let db_path = format!("{}/todos.db", db_dir);

    if !fs::metadata(db_dir).await.is_ok() {
        fs::create_dir_all(db_dir).await?;
        println!("Created directory: {}", db_dir);
    }

    let pool = SqlitePool::connect(&format!("sqlite:{}", db_path)).await?;
    println!("Connected to SQLite database: {}", db_path);

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS todos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            completed BOOLEAN NOT NULL DEFAULT 0
        );
        "#,
    )
    .execute(&pool)
    .await?;

    let app_state = Arc::new(AppState { db: pool });

    let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);


    let app = Router::new()
        .route("/todos", get(list_todos))
        .route("/todos", post(create_todo))
        .route("/todos/:id", get(get_todo))
        .route("/todos/:id", put(update_todo))
        .route("/todos/:id", delete(delete_todo))
        .layer(cors) 
        .with_state(app_state);

    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    println!("Server running on http://localhost:3000");

    serve(listener, app).await?;

    Ok(())
}

// Handlers (your existing logic)
async fn list_todos(
    state: axum::extract::State<Arc<AppState>>,
) -> Json<Vec<Todo>> {
    let todos = sqlx::query_as::<_, Todo>(
        "SELECT * FROM todos ORDER BY id DESC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap();

    Json(todos)
}

async fn create_todo(
    state: axum::extract::State<Arc<AppState>>,
    Json(payload): Json<CreateTodo>,
) -> Json<Todo> {
    let todo = sqlx::query_as::<_, Todo>(
        r#"
        INSERT INTO todos (title, completed)
        VALUES (?, 0)
        RETURNING *
        "#,
    )
    .bind(payload.title)
    .fetch_one(&state.db)
    .await
    .unwrap();

    Json(todo)
}

async fn get_todo(
    state: axum::extract::State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Json<Todo> {
    let todo = sqlx::query_as::<_, Todo>(
        "SELECT * FROM todos WHERE id = ?"
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .unwrap();

    Json(todo)
}

async fn update_todo(
    state: axum::extract::State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateTodo>,
) -> Json<Todo> {
    let current_todo = sqlx::query_as::<_, Todo>(
        "SELECT * FROM todos WHERE id = ?"
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .unwrap();

    let title = payload.title.unwrap_or(current_todo.title);
    let completed = payload.completed.unwrap_or(current_todo.completed);

    let todo = sqlx::query_as::<_, Todo>(
        r#"
        UPDATE todos 
        SET title = ?, 
            completed = ?
        WHERE id = ? 
        RETURNING *
        "#,
    )
    .bind(title)
    .bind(completed)
    .bind(id)
    .fetch_one(&state.db)
    .await
    .unwrap();

    Json(todo)
}

async fn delete_todo(
    state: axum::extract::State<Arc<AppState>>,
    Path(id): Path<i64>,
) {
    sqlx::query("DELETE FROM todos WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .unwrap();
}
