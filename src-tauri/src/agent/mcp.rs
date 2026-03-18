use std::sync::Arc;

use yoagent::mcp::{McpClient, McpToolAdapter};

use crate::error::{AppError, AppResult};
use crate::models::{
    McpServerConfig,
    McpServerStatus,
    McpTransport as McpTransportConfig,
};
use crate::state::AppState;

fn mcp_error(error: impl std::fmt::Display) -> AppError {
    AppError::Mcp(error.to_string())
}

pub async fn connect_server(state: &AppState, config: McpServerConfig) -> AppResult<Vec<String>> {
    let client = match &config.transport {
        McpTransportConfig::Stdio => {
            let args = config
                .args
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            McpClient::connect_stdio(&config.command_or_url, &args, None)
                .await
                .map_err(mcp_error)?
        }
        McpTransportConfig::Http => McpClient::connect_http(&config.command_or_url)
            .await
            .map_err(mcp_error)?,
    };

    let tool_names = client
        .list_tools()
        .await
        .map_err(mcp_error)?
        .into_iter()
        .map(|tool| tool.name)
        .collect::<Vec<_>>();

    state
        .mcp_clients
        .lock()
        .await
        .insert(config.name, Arc::new(tokio::sync::Mutex::new(client)));

    Ok(tool_names)
}

pub async fn disconnect_server(state: &AppState, name: &str) -> AppResult<()> {
    let client = state.mcp_clients.lock().await.remove(name);
    if let Some(client) = client {
        client.lock().await.close().await.map_err(mcp_error)?;
    }
    Ok(())
}

pub async fn get_mcp_tools(state: &AppState) -> AppResult<Vec<Box<dyn yoagent::AgentTool>>> {
    let clients = state
        .mcp_clients
        .lock()
        .await
        .iter()
        .map(|(name, client)| (name.clone(), Arc::clone(client)))
        .collect::<Vec<_>>();

    let mut tools = Vec::new();
    for (server_name, client) in clients {
        let adapters = McpToolAdapter::from_client_with_prefix(client, server_name)
            .await
            .map_err(mcp_error)?;
        tools.extend(
            adapters
                .into_iter()
                .map(|tool| Box::new(tool) as Box<dyn yoagent::AgentTool>),
        );
    }

    Ok(tools)
}

pub async fn get_server_statuses(
    state: &AppState,
    configs: &[McpServerConfig],
) -> Vec<McpServerStatus> {
    let connected = state
        .mcp_clients
        .lock()
        .await
        .keys()
        .cloned()
        .collect::<std::collections::HashSet<_>>();

    configs
        .iter()
        .cloned()
        .map(|config| McpServerStatus {
            connected: connected.contains(&config.name),
            config,
        })
        .collect()
}
