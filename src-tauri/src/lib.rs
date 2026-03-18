pub mod agent;
pub mod commands;
pub mod db;
pub mod error;
pub mod models;
pub mod paste_storage;
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
        state::AppState::new(db_path_str, data_dir.clone()).expect("Failed to initialize AppState"),
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
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            agent::commands::agent_chat,
            agent::commands::agent_stop,
            agent::commands::agent_authorize_tool,
            agent::commands::get_tool_permissions,
            agent::commands::set_tool_permissions,
            agent::commands::get_skills_dir,
            agent::commands::set_skills_dir,
            agent::commands::scan_skills,
            commands::chat::send_message,
            commands::chat::send_message_group,
            commands::chat::resend_message,
            commands::chat::stop_generation,
            commands::chat::generate_version,
            commands::chat::regenerate_message,
            commands::chat::compress_conversation,
            commands::conversation::create_conversation,
            commands::conversation::list_conversations,
            commands::conversation::update_conversation_title,
            commands::conversation::delete_conversation,
            commands::conversation::pin_conversation,
            commands::conversation::update_conversation_assistant,
            commands::conversation::update_conversation_model,
            commands::conversation::generate_conversation_title,
            commands::conversation::get_messages,
            commands::conversation::delete_message,
            commands::conversation::restore_message,
            commands::conversation::delete_messages_after,
            commands::conversation::delete_messages_from,
            commands::conversation::update_message_content,
            commands::conversation::hydrate_paste_content,
            commands::conversation::expand_paste_content,
            commands::conversation::get_paste_blob_content,
            commands::conversation::switch_version,
            commands::conversation::list_versions,
            commands::conversation::list_version_messages,
            commands::conversation::get_version_models,
            commands::conversation::fork_conversation,
            commands::provider::add_provider,
            commands::provider::list_providers,
            commands::provider::update_provider,
            commands::provider::delete_provider,
            commands::provider::fetch_models,
            commands::provider::create_manual_model,
            commands::provider::update_manual_model,
            commands::provider::delete_manual_model,
            commands::provider::update_model_visibility,
            commands::provider::update_provider_models_visibility,
            commands::assistant::create_assistant,
            commands::assistant::list_assistants,
            commands::assistant::update_assistant,
            commands::assistant::delete_assistant,
            commands::search::search_messages,
            commands::search::search_sidebar_results,
            commands::export::export_conversation_markdown,
            commands::export::export_conversation_json,
            commands::settings::get_app_paths,
            commands::settings::open_path,
            commands::settings::pick_directory,
            commands::settings::get_cache_size,
            commands::settings::clear_cache,
            commands::settings::reset_app_data,
            commands::settings::local_backup,
            commands::settings::get_autostart_enabled,
            commands::settings::set_autostart_enabled,
            commands::settings::set_proxy_mode,
            commands::settings::get_proxy_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
