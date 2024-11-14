use axum::{Router, routing::post};
use std::net::SocketAddr;
use crate::server::routes::{activate_license, validate_license, deactivate_license};
use crate::server::database::Database;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db = Database::new().await;

    let app = Router::new()
        .route("/activate", post(activate_license))
        .route("/validate", post(validate_license))
        .route("/deactivate", post(deactivate_license))
        .with_state(db);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("Server listening on http://{}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
