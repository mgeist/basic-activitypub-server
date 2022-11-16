use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

pub fn app() -> Router {
    Router::new().route("/", get(hello))
}

async fn hello() -> impl IntoResponse {
    Html("<h1>Hello</h1>")
}
