pub mod commands;
pub mod db;
pub mod error;
pub mod models;
pub mod providers;
pub mod state;

use std::sync::Arc;

pub fn run() {
    // Determine app data directory
    let data_dir = dirs::data_dir()
        .expect("Failed to get data directory")
        .join("orion-chat");

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");

    let db_path = data_dir.join("orion.db");
    let db_path_str = db_path.to_str().expect("Invalid db path");

    let app_state = Arc::new(
        state::AppState::new(db_path_str).expect("Failed to initialize AppState"),
    );

    // Re-register providers from DB
    let state_clone = app_state.clone();
    let providers = app_state.db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, type, api_key, base_url FROM providers WHERE is_enabled = 1",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    });

    if let Ok(providers) = providers {
        // Use a temporary runtime to register providers at startup
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        for (id, type_str, api_key, api_base) in providers {
            let pt = match type_str.as_str() {
                "anthropic" => models::ProviderType::Anthropic,
                "gemini" => models::ProviderType::Gemini,
                "ollama" => models::ProviderType::Ollama,
                _ => models::ProviderType::OpenaiCompat,
            };
            let _ = rt.block_on(state_clone.register_provider(
                &id,
                &pt,
                api_key.as_deref(),
                api_base.as_deref(),
            ));
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::chat::send_message,
            commands::chat::stop_generation,
            commands::conversation::create_conversation,
            commands::conversation::list_conversations,
            commands::conversation::update_conversation_title,
            commands::conversation::delete_conversation,
            commands::conversation::get_messages,
            commands::provider::add_provider,
            commands::provider::list_providers,
            commands::provider::update_provider,
            commands::provider::delete_provider,
            commands::provider::fetch_models,
            commands::assistant::create_assistant,
            commands::assistant::list_assistants,
            commands::assistant::update_assistant,
            commands::assistant::delete_assistant,
            commands::search::search_messages,
            commands::export::export_conversation_markdown,
            commands::export::export_conversation_json,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
