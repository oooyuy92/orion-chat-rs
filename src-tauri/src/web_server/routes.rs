use std::sync::Arc;

use axum::{
    routing::{get, post, patch, delete},
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
        .route("/conversations/:id/title", patch(handlers::update_conversation_title))
        .route("/conversations/:id", delete(handlers::delete_conversation))
        // Message routes
        .route("/conversations/:id/messages", get(handlers::get_messages))
        .route("/conversations/:id/messages", post(handlers::send_message))
        // Provider routes
        .route("/providers", get(handlers::list_providers))
        .route("/providers", post(handlers::add_provider))
        .route("/providers/:id", patch(handlers::update_provider))
        .route("/providers/:id", delete(handlers::delete_provider))
        .route("/providers/:id/fetch-models", post(handlers::fetch_models_for_provider))
        .route("/providers/:id/models", post(handlers::create_manual_model))
        .route("/providers/:id/models/visibility", patch(handlers::update_provider_models_visibility))
        // Model routes
        .route("/models", get(handlers::list_models))
        .route("/models/:id", patch(handlers::update_manual_model))
        .route("/models/:id", delete(handlers::delete_manual_model))
        .route("/models/:id/visibility", patch(handlers::update_model_visibility))
        // Assistant routes
        .route("/assistants", get(handlers::list_assistants))
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
