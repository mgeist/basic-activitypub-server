mod webfinger;

use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

#[derive(Clone)]
pub struct AppState {
    pub user: String,
    pub domain: String,
}

pub fn app(state: AppState) -> Router<AppState> {
    Router::with_state(state)
        .route("/", get(hello))
        .route("/.well-known/webfinger", get(webfinger::webfinger))
}

async fn hello() -> impl IntoResponse {
    Html("<h1>Hello</h1>")
}
