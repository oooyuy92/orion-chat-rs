use std::sync::Arc;

use orion_chat_rs::{
    models::ProviderType,
    state::AppState,
    web_server,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Starting Orion Chat Web Server...");

    // Determine database path from environment or use default
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| {
            let data_dir = dirs::data_dir()
                .expect("Failed to get data directory")
                .join("orion-chat");

            // Create directory if it doesn't exist
            std::fs::create_dir_all(&data_dir)
                .expect("Failed to create data directory");

            data_dir.join("orion.db")
                .to_str()
                .expect("Invalid db path")
                .to_string()
        });

    println!("Using database: {}", db_path);

    // Determine data directory
    let data_dir = std::env::var("DATA_DIR")
        .map(|p| std::path::PathBuf::from(p))
        .unwrap_or_else(|_| {
            dirs::data_dir()
                .expect("Failed to get data directory")
                .join("orion-chat")
        });

    // Initialize database and app state
    let app_state = Arc::new(AppState::new(&db_path, data_dir.clone())?);

    // Load and register providers from database
    println!("Loading providers from database...");
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
    })?;

    for (id, type_str, api_key, api_base) in providers {
        let pt = match type_str.as_str() {
            "anthropic" => ProviderType::Anthropic,
            "gemini" => ProviderType::Gemini,
            "ollama" => ProviderType::Ollama,
            _ => ProviderType::OpenaiCompat,
        };

        match app_state.register_provider(
            &id,
            &pt,
            api_key.as_deref(),
            api_base.as_deref(),
        ).await {
            Ok(_) => println!("Registered provider: {} ({})", id, type_str),
            Err(e) => eprintln!("Failed to register provider {}: {}", id, e),
        }
    }

    // Get static directory from environment or use None
    let static_dir = std::env::var("STATIC_DIR").ok();

    // Create Axum app
    let app = web_server::create_app(
        app_state,
        static_dir.as_deref(),
    );

    // Bind to port 28080
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "28080".to_string())
        .parse::<u16>()
        .expect("Invalid PORT value");

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));

    println!("Web server listening on http://{}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
