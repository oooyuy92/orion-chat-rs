use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PermissionLevel {
    Auto,
    Ask,
    Deny,
}

impl Default for PermissionLevel {
    fn default() -> Self {
        Self::Ask
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissions(pub HashMap<String, PermissionLevel>);

impl ToolPermissions {
    pub fn with_defaults() -> Self {
        let mut map = HashMap::new();
        map.insert("read_file".to_string(), PermissionLevel::Auto);
        map.insert("list_files".to_string(), PermissionLevel::Auto);
        map.insert("search".to_string(), PermissionLevel::Auto);
        map.insert("edit_file".to_string(), PermissionLevel::Ask);
        map.insert("write_file".to_string(), PermissionLevel::Ask);
        map.insert("bash".to_string(), PermissionLevel::Ask);
        Self(map)
    }
}

impl Default for ToolPermissions {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AuthAction {
    Allow,
    AllowSession,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum McpTransport {
    Stdio,
    Http,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransport,
    pub command_or_url: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerStatus {
    pub config: McpServerConfig,
    pub connected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_permissions_default_policy_matches_agent_settings_seed() {
        let defaults = ToolPermissions::default();

        assert_eq!(defaults.0.get("read_file"), Some(&PermissionLevel::Auto));
        assert_eq!(defaults.0.get("list_files"), Some(&PermissionLevel::Auto));
        assert_eq!(defaults.0.get("search"), Some(&PermissionLevel::Auto));
        assert_eq!(defaults.0.get("edit_file"), Some(&PermissionLevel::Ask));
        assert_eq!(defaults.0.get("write_file"), Some(&PermissionLevel::Ask));
        assert_eq!(defaults.0.get("bash"), Some(&PermissionLevel::Ask));
    }
}
