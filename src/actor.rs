use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;

use crate::AppState;

const PUB_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA5TGMtbnYItWPsdjCZsBT
PdsjLv0xn6JoMd639v2+7XuMMO9zCrWNFXL81OUTMr88RRHblk5TX+zXcKKeC9h5
Pm0pJst2IsKylw4HgIkJ8sRFfNg1QMCJ0pES6RQkQG6o3zRuYkR1NFxZ5mk8HtxO
Fg+glCVwTu3IcXaMe2YB1+NVga7ZBrWqtMuGOWgaI/R2JKBC4aHSvuZVUiDwjXPQ
Js0pHbglxt3DHQ7COmOLn2t6vN703M+44TmCf7pAwZ9V4ZoLrYDb/OnfeD5QFB4K
jg44TkhIR7MFeZj+DVFpGc/qk7hT4O3lVPjZAzgHphAiGA4q1Dc/xKx3I3Pqjcpc
5QIDAQAB
-----END PUBLIC KEY-----"#;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActorResponse {
    #[serde(rename = "@context")]
    context: Vec<String>,
    id: String,
    #[serde(rename = "type")]
    kind: String,
    preferred_username: String,
    inbox: String,
    public_key: ActorPublicKey,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActorPublicKey {
    id: String,
    owner: String,
    public_key_pem: String,
}

pub(crate) async fn actor(State(state): State<AppState>) -> impl IntoResponse {
    let public_key = ActorPublicKey {
        id: format!("https://{}/actor#main-key", state.domain),
        owner: format!("https://{}/actor", state.domain),
        public_key_pem: PUB_KEY.to_string(),
    };

    let actor_response = ActorResponse {
        context: vec![
            "https://www.w3.org/ns/activitystreams".to_string(),
            "https://w3id.org/security/v1".to_string(),
        ],
        id: format!("https://{}/actor", state.domain),
        kind: "Person".to_string(),
        preferred_username: state.user,
        inbox: format!("https://{}/inbox", state.domain),
        public_key,
    };

    Json(actor_response)
}
