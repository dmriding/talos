use axum::{Router, routing::post};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tracing_subscriber;
use crate::server::routes::{activate_license, validate_license, deactivate_license};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Define routes
    let app = Router::new()
        .route("/activate", post(activate_license))
        .route("/validate", post(validate_license))
        .route("/deactivate", post(deactivate_license))
        .layer(ServiceBuilder::new());

    // Define server address
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("Listening on http://{}", addr);

    // Start server
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
