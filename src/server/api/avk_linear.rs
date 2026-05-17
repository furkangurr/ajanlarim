//! AVK Linear iş kuyruğu endpoint — FUR-4160.
//!
//! `GET /api/avk/linear-queue` Linear GraphQL API'sine (https://api.linear.app/graphql)
//! `LINEAR_API_KEY` env'i ile sorgu atar; aktif (started) ve sıradaki
//! (unstarted) durum tipindeki issue'ları priority sıralı döner.
//!
//! Sunucu tarafı: API key client'a sızdırılmaz, frontend sadece backend'i
//! çağırır. ENV yoksa 503 + `not_configured` etiketi (UI bilgilendirici nota
//! döner, mock göstermez — bu kuyruk gerçek olmazsa anlam taşımıyor).
//!
//! 5sn timeout — Linear API genelde <1sn döner; sustained dashboard refresh
//! kadansı (30s) sırasında stale veriden iyi.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

use super::AppState;

const LINEAR_GRAPHQL_URL: &str = "https://api.linear.app/graphql";
const LINEAR_TIMEOUT: Duration = Duration::from_secs(5);
const QUEUE_LIMIT: u32 = 30;

const QUERY: &str = r#"
query AvkQueue($first: Int!) {
  issues(
    first: $first
    filter: { state: { type: { in: ["started", "unstarted"] } } }
    orderBy: priority
  ) {
    nodes {
      id
      identifier
      title
      priority
      priorityLabel
      state { name type }
      assignee { name }
      team { key }
      url
      updatedAt
    }
  }
}
"#;

#[derive(Debug, Serialize, Clone)]
pub struct LinearIssue {
    pub id: String,
    pub identifier: String,
    pub title: String,
    pub priority: u8,
    pub priority_label: String,
    pub state_name: String,
    pub state_type: String,
    pub assignee: Option<String>,
    pub team_key: Option<String>,
    pub url: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct LinearQueueResponse {
    pub active: Vec<LinearIssue>,
    pub backlog: Vec<LinearIssue>,
    pub total: usize,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'static str>,
}

pub async fn get_avk_linear_queue(State(_state): State<Arc<AppState>>) -> Response {
    let Some(api_key) = load_api_key() else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "LINEAR_API_KEY env yapılandırılmamış",
            Some("not_configured"),
        );
    };

    match fetch_linear(&api_key).await {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => error_response(StatusCode::BAD_GATEWAY, &e, Some("upstream_error")),
    }
}

fn load_api_key() -> Option<String> {
    std::env::var("LINEAR_API_KEY")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

async fn fetch_linear(api_key: &str) -> Result<LinearQueueResponse, String> {
    let body = serde_json::json!({
        "query": QUERY,
        "variables": { "first": QUEUE_LIMIT },
    });

    let client = reqwest::Client::builder()
        .timeout(LINEAR_TIMEOUT)
        .build()
        .map_err(|e| format!("reqwest client build: {e}"))?;
    let resp = client
        .post(LINEAR_GRAPHQL_URL)
        .header("Authorization", api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Linear unreachable: {e}"))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Linear body read: {e}"))?;
    if !status.is_success() {
        return Err(format!("Linear HTTP {status}: {}", truncate(&text, 200)));
    }

    let parsed: Value = serde_json::from_str(&text).map_err(|e| format!("Linear parse: {e}"))?;
    if let Some(errors) = parsed.get("errors") {
        return Err(format!(
            "Linear GraphQL errors: {}",
            truncate(&errors.to_string(), 200)
        ));
    }

    let nodes = parsed
        .get("data")
        .and_then(|d| d.get("issues"))
        .and_then(|i| i.get("nodes"))
        .and_then(|n| n.as_array())
        .ok_or_else(|| "Linear response: data.issues.nodes missing".to_string())?;

    let issues: Vec<LinearIssue> = nodes.iter().filter_map(parse_issue).collect();

    let mut active = Vec::new();
    let mut backlog = Vec::new();
    for issue in &issues {
        if issue.state_type == "started" {
            active.push(issue.clone());
        } else if issue.state_type == "unstarted" {
            backlog.push(issue.clone());
        }
    }
    let total = issues.len();

    Ok(LinearQueueResponse {
        active,
        backlog,
        total,
    })
}

fn parse_issue(node: &Value) -> Option<LinearIssue> {
    let id = node.get("id")?.as_str()?.to_string();
    let identifier = node.get("identifier")?.as_str()?.to_string();
    let title = node.get("title")?.as_str()?.to_string();
    let priority = node.get("priority").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let priority_label = node
        .get("priorityLabel")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let state = node.get("state")?;
    let state_name = state.get("name")?.as_str()?.to_string();
    let state_type = state.get("type")?.as_str()?.to_string();
    let assignee = node
        .get("assignee")
        .and_then(|a| a.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());
    let team_key = node
        .get("team")
        .and_then(|t| t.get("key"))
        .and_then(|k| k.as_str())
        .map(|s| s.to_string());
    let url = node
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let updated_at = node
        .get("updatedAt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(LinearIssue {
        id,
        identifier,
        title,
        priority,
        priority_label,
        state_name,
        state_type,
        assignee,
        team_key,
        url,
        updated_at,
    })
}

fn error_response(status: StatusCode, msg: &str, kind: Option<&'static str>) -> Response {
    (
        status,
        Json(ErrorBody {
            error: msg.to_string(),
            kind,
        }),
    )
        .into_response()
}

fn truncate(s: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (idx, ch) in s.chars().enumerate() {
        if idx >= max_chars {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}
