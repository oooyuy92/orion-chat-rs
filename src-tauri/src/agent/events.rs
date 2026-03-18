use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use yoagent::{AgentEvent, AgentMessage, Content, Message, StreamDelta, ToolResult};

use crate::agent::permissions::ChatEventEmitter;
use crate::agent::storage::{insert_tool_call_result, insert_tool_call_start};
use crate::db::Database;
use crate::models::ChatEvent;

pub async fn handle_agent_event(
    event: AgentEvent,
    emit_event: &ChatEventEmitter,
    db: &Database,
    conversation_id: &str,
    assistant_message_id: &str,
    tool_message_ids: &Arc<Mutex<HashMap<String, String>>>,
) {
    match event {
        AgentEvent::MessageUpdate {
            delta: StreamDelta::Text { delta },
            ..
        } => {
            emit_event(ChatEvent::Delta {
                message_id: assistant_message_id.to_string(),
                content: delta,
            });
        }
        AgentEvent::MessageUpdate {
            delta: StreamDelta::Thinking { delta },
            ..
        } => {
            emit_event(ChatEvent::Reasoning {
                message_id: assistant_message_id.to_string(),
                content: delta,
            });
        }
        AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            args,
        } => {
            if let Ok(message_id) = insert_tool_call_start(
                db,
                conversation_id,
                &tool_call_id,
                &tool_name,
                &args.to_string(),
            ) {
                tool_message_ids
                    .lock()
                    .await
                    .insert(tool_call_id.clone(), message_id.clone());
                emit_event(ChatEvent::ToolCallStart {
                    message_id,
                    tool_call_id,
                    tool_name,
                    args: args.to_string(),
                });
            }
        }
        AgentEvent::ToolExecutionUpdate {
            tool_call_id,
            partial_result,
            ..
        } => {
            let message_id = tool_message_ids.lock().await.get(&tool_call_id).cloned();
            if let Some(message_id) = message_id {
                emit_event(ChatEvent::ToolCallUpdate {
                    message_id,
                    tool_call_id,
                    partial_result: render_tool_result(&partial_result),
                });
            }
        }
        AgentEvent::ToolExecutionEnd {
            tool_call_id,
            tool_name,
            result,
            is_error,
            ..
        } => {
            if let Ok(message_id) = insert_tool_call_result(
                db,
                conversation_id,
                &tool_call_id,
                &tool_name,
                &render_tool_result(&result),
                is_error,
            ) {
                emit_event(ChatEvent::ToolCallEnd {
                    message_id,
                    tool_call_id: tool_call_id.clone(),
                    result: render_tool_result(&result),
                    is_error,
                });
            }
            tool_message_ids.lock().await.remove(&tool_call_id);
        }
        AgentEvent::MessageEnd {
            message:
                AgentMessage::Llm(Message::Assistant {
                    stop_reason: yoagent::StopReason::Error,
                    error_message,
                    ..
                }),
        } => {
            emit_event(ChatEvent::Error {
                message_id: assistant_message_id.to_string(),
                message: error_message.unwrap_or_else(|| "Agent error".to_string()),
            });
        }
        AgentEvent::InputRejected { reason } => {
            emit_event(ChatEvent::Error {
                message_id: assistant_message_id.to_string(),
                message: reason,
            });
        }
        _ => {}
    }
}

fn render_tool_result(result: &ToolResult) -> String {
    let rendered = result
        .content
        .iter()
        .map(render_content)
        .collect::<Vec<_>>()
        .join("\n");

    if rendered.is_empty() {
        result.details.to_string()
    } else {
        rendered
    }
}

fn render_content(content: &Content) -> String {
    match content {
        Content::Text { text } => text.clone(),
        Content::Thinking { thinking, .. } => thinking.clone(),
        Content::ToolCall {
            id,
            name,
            arguments,
        } => format!("{name} ({id}): {arguments}"),
        Content::Image { mime_type, .. } => format!("<image:{mime_type}>"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;
    use crate::models::Conversation;

    fn setup_db() -> Database {
        let db = Database::new(":memory:").unwrap();
        db.with_conn(|conn| {
            crate::db::conversations::create(
                conn,
                &Conversation {
                    id: "conv-1".into(),
                    title: "Test".into(),
                    assistant_id: None,
                    model_id: None,
                    is_pinned: false,
                    sort_order: 0,
                    created_at: "2026-03-18T00:00:00Z".into(),
                    updated_at: "2026-03-18T00:00:00Z".into(),
                },
            )?;
            Ok(())
        })
        .unwrap();
        db
    }

    #[tokio::test]
    async fn test_message_update_maps_to_delta_event() {
        let db = setup_db();
        let events = Arc::new(StdMutex::new(Vec::new()));
        let emitter: ChatEventEmitter = Arc::new({
            let events = events.clone();
            move |event| events.lock().unwrap().push(event)
        });
        let tool_ids = Arc::new(Mutex::new(HashMap::new()));

        handle_agent_event(
            AgentEvent::MessageUpdate {
                message: AgentMessage::from(Message::user("hi")),
                delta: StreamDelta::Text {
                    delta: "hello".into(),
                },
            },
            &emitter,
            &db,
            "conv-1",
            "assistant-1",
            &tool_ids,
        )
        .await;

        assert!(matches!(
            &events.lock().unwrap()[0],
            ChatEvent::Delta { message_id, content } if message_id == "assistant-1" && content == "hello"
        ));
    }

    #[tokio::test]
    async fn test_tool_events_persist_and_emit_chat_events() {
        let db = setup_db();
        let events = Arc::new(StdMutex::new(Vec::new()));
        let emitter: ChatEventEmitter = Arc::new({
            let events = events.clone();
            move |event| events.lock().unwrap().push(event)
        });
        let tool_ids = Arc::new(Mutex::new(HashMap::new()));

        handle_agent_event(
            AgentEvent::ToolExecutionStart {
                tool_call_id: "call-1".into(),
                tool_name: "bash".into(),
                args: serde_json::json!({ "command": "echo hi" }),
            },
            &emitter,
            &db,
            "conv-1",
            "assistant-1",
            &tool_ids,
        )
        .await;

        handle_agent_event(
            AgentEvent::ToolExecutionEnd {
                tool_call_id: "call-1".into(),
                tool_name: "bash".into(),
                result: ToolResult {
                    content: vec![Content::Text { text: "hi".into() }],
                    details: serde_json::json!({}),
                },
                is_error: false,
            },
            &emitter,
            &db,
            "conv-1",
            "assistant-1",
            &tool_ids,
        )
        .await;

        let events = events.lock().unwrap();
        assert!(events.iter().any(|event| matches!(event, ChatEvent::ToolCallStart { tool_name, .. } if tool_name == "bash")));
        assert!(events.iter().any(|event| matches!(event, ChatEvent::ToolCallEnd { result, .. } if result == "hi")));

        let tool_message_count: i64 = db
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT COUNT(*) FROM messages WHERE tool_call_id = 'call-1'",
                    [],
                    |row| row.get(0),
                )
                .map_err(Into::into)
            })
            .unwrap();
        assert_eq!(tool_message_count, 2);
    }
}
