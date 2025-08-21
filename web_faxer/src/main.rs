use axum::{
    Form, Router,
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
};
use chrono::{DateTime, Days, Duration, Local, NaiveDateTime, NaiveTime, TimeDelta};
use serde::Deserialize;
use tokio::sync::Mutex;
use types::FaxMessage;

use std::{collections::HashMap, sync::Arc};
use std::{fs::read_to_string, net::SocketAddr};

#[tokio::main]
async fn main() {
    let users = Arc::new(Mutex::new(get_users()));
    let index = include_str!("../templates/index.html").to_string();
    let message_form = include_str!("../templates/message_form.html").to_string();
    let success = include_str!("../templates/success.html").to_string();
    let timeout = include_str!("../templates/timeout.html").to_string();

    let pages = Pages {
        index,
        message_form,
        timeout,
        success,
    };

    let app_state = AppState { users, pages };

    let app = Router::new()
        .route("/", get(index_page))
        .route(
            "/message/{uuid}",
            get(message_form_page).post(message_form_post),
        )
        .route("/timeout", get(timeout_page))
        .with_state(app_state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

fn get_users() -> HashMap<String, User> {
    let mut map = HashMap::new();
    for line in read_to_string("user_list.txt")
        .expect("no user list")
        .lines()
        .filter(|x| !x.is_empty())
    {
        let mut user = line.split(":");
        let uuid = user.next().unwrap().to_string();
        let name = user.next().unwrap().to_string();
        map.insert(
            uuid,
            User {
                name,
                last_message: Local::now() - Duration::weeks(1),
            },
        );
    }

    map
}

async fn index_page(State(state): State<AppState>) -> impl IntoResponse {
    return Html(state.pages.index);
}

async fn message_form_page(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut users = state.users.lock().await;
    if let Some(user) = users.get_mut(&uuid) {
        if Local::now().date_naive() == user.last_message.date_naive() {
            return Ok(timeout_html(&state));
        }
        return Ok(Html(
            state.pages.message_form.replace("{{ name }}", &user.name),
        ));
    }

    Err(StatusCode::NOT_FOUND)
}

async fn message_form_post(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
    ConnectInfo(addrs): ConnectInfo<SocketAddr>,
    Form(form): Form<MessagePost>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut users = state.users.lock().await;
    if let Some(user) = users.get_mut(&uuid) {
        // Send to timeout if user has already sent a message today
        if Local::now().date_naive() == user.last_message.date_naive() {
            return Ok(timeout_html(&state));
        }

        // 48 is line length
        if form.message.len() > 48 * 20 {
            return Ok(Html("ono that is too long".into()));
        }

        let resp = send_fax_to_printer(form.message, &user.name, addrs.to_string()).await;
        if let Err(msg) = resp {
            return Ok(Html(msg.to_string()));
        }

        user.last_message = Local::now();
        return Ok(Html(state.pages.success));
    }

    Ok(Html("You don't exist :(".to_string()))
}

fn timeout_html(state: &AppState) -> Html<String> {
    let now = Local::now().naive_local();
    let tomorrow = NaiveDateTime::new(
        now.date().checked_add_days(Days::new(1)).unwrap(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    );
    let remaining_time = tomorrow - now;
    let hours = remaining_time.num_hours();
    let mins = (remaining_time - TimeDelta::hours(hours)).num_minutes();
    dbg!(hours, mins);
    return Html(
        state
            .pages
            .timeout
            .replace("{{ hours }}", &hours.to_string())
            .replace("{{ mins }}", &mins.to_string()),
    );
}

async fn send_fax_to_printer(
    message: String,
    username: &str,
    ip: String,
) -> Result<bool, reqwest::Error> {
    let payload = FaxMessage {
        time: Local::now(),
        message,
        from: username.to_string(),
        ip: ip.to_string(),
    };

    let resp = reqwest::Client::new()
        .post("http://kotpi.tabby-wall.ts.net:3000/fax")
        .json(&payload)
        .send()
        .await?;
    Ok(resp.status().is_success())
}

async fn timeout_page(State(state): State<AppState>) -> impl IntoResponse {
    return Html(state.pages.timeout);
}

#[derive(Deserialize, Clone)]
struct MessagePost {
    message: String,
}

#[derive(Clone)]
struct User {
    name: String,
    last_message: DateTime<Local>,
}

#[derive(Clone)]
struct AppState {
    users: Arc<Mutex<HashMap<String, User>>>,
    pages: Pages,
}

#[derive(Clone)]
struct Pages {
    index: String,
    message_form: String,
    timeout: String,
    success: String,
}
