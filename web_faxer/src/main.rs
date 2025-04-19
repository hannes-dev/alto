use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
};
use sqlx::{Pool, Sqlite, SqlitePool, prelude::FromRow};

#[tokio::main]
async fn main() {
    let db_pool = SqlitePool::connect_lazy("sqlite:web_faxer.db").unwrap();
    make_db(&db_pool).await;

    let index = include_str!("../templates/index.html").to_string();
    let message_form = include_str!("../templates/message_form.html").to_string();
    let timeout = include_str!("../templates/timeout.html").to_string();

    let pages = Pages {
        index,
        message_form,
        timeout,
    };

    let app_state = AppState { db_pool, pages };

    let app = Router::new()
        .route("/", get(index_page))
        .route("/message/{uuid}", get(message_form_page))
        .route("/timeout", get(timeout_page))
        .with_state(app_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index_page(State(state): State<AppState>) -> impl IntoResponse {
    return Html(state.pages.index);
}

async fn message_form_page(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let user = sqlx::query_as::<_, User>("SELECT name FROM users WHERE id = ?")
        .bind(uuid)
        .fetch_optional(&state.db_pool)
        .await
        .unwrap();

    match user {
        Some(user) => {
            return Ok(Html(
                state.pages.message_form.replace("{{ name }}", &user.name),
            ));
        }
        None => return Err(StatusCode::NOT_FOUND),
    };
}

#[derive(Debug, FromRow)]
struct User {
    name: String,
}

async fn timeout_page(State(state): State<AppState>) -> impl IntoResponse {
    return Html(state.pages.timeout);
}

async fn make_db(pool: &Pool<Sqlite>) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id   TEXT PRIMARY KEY, -- UUID
            name TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
}

#[derive(Clone)]
struct AppState {
    db_pool: Pool<Sqlite>,
    pages: Pages,
}

#[derive(Clone)]
struct Pages {
    index: String,
    message_form: String,
    timeout: String,
}
