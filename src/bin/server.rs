use std::{env, net::SocketAddr};

use basic_activitypub_server::{app, AppState};

#[tokio::main]
async fn main() {
    let user = env::var("AP_USER").unwrap();
    let domain = env::var("AP_DOMAIN").unwrap();

    let app = app(AppState { user, domain });

    let address = SocketAddr::from(([0, 0, 0, 0], 8080));

    println!("Server started at {}", address);

    axum::Server::bind(&address)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
