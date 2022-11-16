use std::net::SocketAddr;

use basic_activitypub_server::app;

#[tokio::main]
async fn main() {
    let app = app();

    let address = SocketAddr::from(([0, 0, 0, 0], 8080));

    println!("Server started at {}", address);

    axum::Server::bind(&address)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
