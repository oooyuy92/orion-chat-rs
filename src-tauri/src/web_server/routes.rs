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
        .route("/conversations/:id/pin", patch(handlers::pin_conversation))
        .route("/conversations/:id/assistant", patch(handlers::update_conversation_assistant))
        .route("/conversations/:id/model", patch(handlers::update_conversation_model))
        .route("/conversations/:id/generate-title", post(handlers::generate_conversation_title))
        .route("/conversations/:id/fork", post(handlers::fork_conversation))
        // Message routes
        .route("/conversations/:id/messages", get(handlers::get_messages))
        .route("/conversations/:id/messages", post(handlers::send_message))
        .route("/conversations/:id/messages/delete-after", post(handlers::delete_messages_after))
        .route("/conversations/:id/messages/delete-from", post(handlers::delete_messages_from))
        .route("/conversations/:id/stop", post(handlers::stop_generation))
        .route("/conversations/:id/resend", post(handlers::resend_message))
        .route("/messages/:id", delete(handlers::delete_message))
        .route("/messages/:id/restore", post(handlers::restore_message))
        .route("/messages/:id/content", patch(handlers::update_message_content))
        .route("/messages/:id/switch-version", post(handlers::switch_version))
        .route("/messages/:id/versions", get(handlers::list_versions))
        .route("/messages/:id/version-messages", get(handlers::list_version_messages))
        .route("/messages/:id/version-models", get(handlers::get_version_models))
        .route("/messages/:id/stream", get(handlers::stream_message))
        .route("/messages/:id/regenerate", post(handlers::regenerate_message))
        .route("/messages/:id/generate-version", post(handlers::generate_message_version))
        // Paste routes
        .route("/paste/:id", get(handlers::get_paste_blob_content))
        .route("/paste/hydrate", post(handlers::hydrate_paste_content))
        .route("/paste/expand", post(handlers::expand_paste_content))
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
        .route("/assistants", post(handlers::create_assistant))
        .route("/assistants/:id", patch(handlers::update_assistant))
        .route("/assistants/:id", delete(handlers::delete_assistant))
        // Search routes
        .route("/search/messages", get(handlers::search_messages))
        .route("/search/sidebar", get(handlers::search_sidebar_results))
        // Export routes
        .route("/conversations/:id/export/markdown", get(handlers::export_conversation_markdown))
        .route("/conversations/:id/export/json", get(handlers::export_conversation_json))
        // Settings routes
        .route("/settings/proxy", get(handlers::get_proxy_mode))
        .route("/settings/proxy", post(handlers::set_proxy_mode))
        .route("/settings/reset", post(handlers::reset_app_data))
        .route("/settings/cache-size", get(handlers::get_cache_size))
        .route("/settings/clear-cache", post(handlers::clear_cache))
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
