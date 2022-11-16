use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
struct WebfingerResponse {
    subject: String,
    links: Vec<WebfingerLink>,
}

#[derive(Serialize)]
struct WebfingerLink {
    rel: String,
    #[serde(rename = "type")]
    kind: String,
    href: String,
}

pub(crate) async fn webfinger(State(state): State<AppState>) -> impl IntoResponse {
    let self_link = WebfingerLink {
        rel: "self".to_string(),
        kind: "application/activity+json".to_string(),
        href: format!("https://{}/actor", state.domain),
    };

    let webfinger_response = WebfingerResponse {
        subject: format!("acct:{}@{}", state.user, state.domain),
        links: vec![self_link],
    };

    Json(webfinger_response)
}
