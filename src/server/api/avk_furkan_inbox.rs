//! Furkan inbox endpoint — FUR-4170.
//!
//! `GET /api/avk/furkan-inbox[?limit=50&unread_only=false]` — `agentmemory`
//! MCP'den `memory_signal_read agentId=furkan` çağrısı ile ajan→Furkan
//! mesajlarını döner. Furkan dashboard'unun bildirim ve mesaj görüntüleme
//! widget'ı bu endpoint'i 30s polling ile çeker.
//!
//! ## Davranış
//!
//! MCP'nin `memory_signal_read` doğası gereği "delivered" mesajları "read"
//! olarak işaretler — yani widget her açıldığında bekleyen signal'ler
//! otomatik okundu sayılır. UI 'okunmamış' badge'i son fetch zamanına göre
//! lokal state'te tutar.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

use super::AppState;

const MCP_URL: &str = "http://localhost:3111/agentmemory/mcp/call";
const MCP_TIMEOUT: Duration = Duration::from_secs(4);
const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 200;
const AGENT_ID: &str = "furkan";

#[derive(Deserialize)]
pub struct InboxQuery {
    pub limit: Option<u32>,
    #[serde(default)]
    pub unread_only: Option<bool>,
    pub thread_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InboxSignal {
    pub id: String,
    pub from: String,
    pub to: String,
    pub r#type: String,
    pub content: String,
    pub thread_id: String,
    pub created_at: String,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InboxResponse {
    pub agent_id: &'static str,
    pub count: usize,
    pub signals: Vec<InboxSignal>,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

pub async fn get_avk_furkan_inbox(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<InboxQuery>,
) -> Response {
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    match fetch_inbox(
        limit,
        query.unread_only.unwrap_or(false),
        query.thread_id.as_deref(),
    )
    .await
    {
        Ok(signals) => Json(InboxResponse {
            agent_id: AGENT_ID,
            count: signals.len(),
            signals,
        })
        .into_response(),
        Err(e) => error_response(StatusCode::BAD_GATEWAY, &format!("MCP fail: {e}")),
    }
}

async fn fetch_inbox(
    limit: u32,
    unread_only: bool,
    thread_id: Option<&str>,
) -> Result<Vec<InboxSignal>, String> {
    let mut args = serde_json::json!({
        "agentId": AGENT_ID,
        "limit": limit,
    });
    if unread_only {
        // MCP unreadOnly string olarak bekliyor (önceki sustained turn deneyimi).
        args["unreadOnly"] = serde_json::Value::String("true".to_string());
    }
    if let Some(tid) = thread_id {
        if !tid.is_empty() {
            args["threadId"] = serde_json::Value::String(tid.to_string());
        }
    }
    let body = serde_json::json!({
        "name": "memory_signal_read",
        "arguments": args,
    });

    let client = reqwest::Client::builder()
        .timeout(MCP_TIMEOUT)
        .build()
        .map_err(|e| format!("reqwest build: {e}"))?;
    let resp = client
        .post(MCP_URL)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("MCP unreachable: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("MCP status {}", resp.status()));
    }

    let outer: Value = resp.json().await.map_err(|e| format!("outer parse: {e}"))?;
    let inner_text = outer
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| "content[0].text missing".to_string())?;
    let inner: Value = serde_json::from_str(inner_text).map_err(|e| format!("inner parse: {e}"))?;

    let arr = inner
        .get("signals")
        .and_then(|s| s.as_array())
        .ok_or_else(|| "signals[] missing".to_string())?;

    Ok(arr.iter().filter_map(parse_signal).collect())
}

fn parse_signal(node: &Value) -> Option<InboxSignal> {
    let id = node.get("id")?.as_str()?.to_string();
    let from = node.get("from")?.as_str()?.to_string();
    let to = node
        .get("to")
        .and_then(|v| v.as_str())
        .unwrap_or(AGENT_ID)
        .to_string();
    let r#type = node
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("chat")
        .to_string();
    let content = node
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let thread_id = node
        .get("threadId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let created_at = node
        .get("createdAt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let status = node
        .get("status")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Some(InboxSignal {
        id,
        from,
        to,
        r#type,
        content,
        thread_id,
        created_at,
        status,
    })
}

fn error_response(status: StatusCode, msg: &str) -> Response {
    (
        status,
        Json(ErrorBody {
            error: msg.to_string(),
        }),
    )
        .into_response()
}
