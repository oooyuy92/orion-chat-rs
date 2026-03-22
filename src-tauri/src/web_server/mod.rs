pub mod handlers;
pub mod middleware;
pub mod routes;

use std::sync::Arc;

use axum::Router;

use crate::state::AppState;

pub fn create_app(state: Arc<AppState>, static_dir: Option<&str>) -> Router {
    routes::create_router(state, static_dir)
}
