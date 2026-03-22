use std::sync::Arc;

use axum::{
    routing::{get, post, put, delete},
    Router,
};
use tower_http::services::ServeDir;

use crate::state::AppState;

use super::{handlers, middleware};

pub fn create_router(state: Arc<AppState>, static_dir: Option<&str>) -> Router {
    let api_routes = Router::new()
        // Conversation routes
        .route("/conversations", get(handlers::list_conversations))
        .route("/conversations", post(handlers::create_conversation))
        .route("/conversations/:id", put(handlers::update_conversation_title))
        .route("/conversations/:id", delete(handlers::delete_conversation))
        // Message routes
        .route("/conversations/:id/messages", get(handlers::get_messages))
        .route("/conversations/:id/messages", post(handlers::send_message))
        // Provider routes
        .route("/providers", get(handlers::list_providers))
        // Assistant routes
        .route("/assistants", get(handlers::list_assistants))
        // Model routes
        .route("/models", get(handlers::list_models))
        .with_state(state);

    let mut app = Router::new()
        .nest("/api", api_routes)
        .layer(middleware::cors_layer());

    // Add static file serving if directory is provided
    if let Some(dir) = static_dir {
        app = app.fallback_service(ServeDir::new(dir));
    }

    app
}
