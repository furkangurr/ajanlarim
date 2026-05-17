//! AVK roadmap endpoint — FUR-4165.
//!
//! `GET /api/avk/roadmap` Linear GraphQL'dan initiatives + alt projeleri
//! (progress + state) çeker; dashboard widget'ı progress bar şeridi olarak
//! gösterir. `LINEAR_API_KEY` env reuse (FUR-4160 pattern).
//!
//! 5sn timeout, Linear rate limit cömert (5000 req/saat / app).

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
const FIRST_INITIATIVES: u32 = 10;
const FIRST_PROJECTS: u32 = 30;

const QUERY: &str = r#"
query AvkRoadmap($initLimit: Int!, $projLimit: Int!) {
  initiatives(first: $initLimit) {
    nodes {
      id
      name
      status
      targetDate
      updatedAt
      projects(first: $projLimit) {
        nodes {
          id
          name
          progress
          state
          targetDate
          url
        }
      }
    }
  }
}
"#;

#[derive(Debug, Serialize, Clone)]
pub struct RoadmapProject {
    pub id: String,
    pub name: String,
    /// 0.0 - 1.0 arası tamamlanma oranı (Linear progress float).
    pub progress: f64,
    /// Linear project state: `backlog` / `planned` / `started` / `paused` /
    /// `completed` / `canceled`.
    pub state: String,
    pub target_date: Option<String>,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct RoadmapInitiative {
    pub id: String,
    pub name: String,
    pub status: String,
    pub target_date: Option<String>,
    pub updated_at: String,
    pub projects: Vec<RoadmapProject>,
    /// Initiative progress = projects.progress aritmetik ortalaması (0-1).
    pub avg_progress: f64,
}

#[derive(Debug, Serialize)]
pub struct RoadmapResponse {
    pub initiatives: Vec<RoadmapInitiative>,
    pub total_initiatives: usize,
    pub total_projects: usize,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'static str>,
}

pub async fn get_avk_roadmap(State(_state): State<Arc<AppState>>) -> Response {
    let Some(api_key) = load_api_key() else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "LINEAR_API_KEY env yapılandırılmamış",
            Some("not_configured"),
        );
    };

    match fetch_roadmap(&api_key).await {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => error_response(StatusCode::BAD_GATEWAY, &e, Some("upstream_error")),
    }
}

fn load_api_key() -> Option<String> {
    std::env::var("LINEAR_API_KEY")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

async fn fetch_roadmap(api_key: &str) -> Result<RoadmapResponse, String> {
    let body = serde_json::json!({
        "query": QUERY,
        "variables": { "initLimit": FIRST_INITIATIVES, "projLimit": FIRST_PROJECTS },
    });

    let client = reqwest::Client::builder()
        .timeout(LINEAR_TIMEOUT)
        .build()
        .map_err(|e| format!("reqwest build: {e}"))?;
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
        .and_then(|d| d.get("initiatives"))
        .and_then(|i| i.get("nodes"))
        .and_then(|n| n.as_array())
        .ok_or_else(|| "Linear response: data.initiatives.nodes missing".to_string())?;

    let initiatives: Vec<RoadmapInitiative> = nodes.iter().filter_map(parse_initiative).collect();
    let total_projects: usize = initiatives.iter().map(|i| i.projects.len()).sum();

    Ok(RoadmapResponse {
        total_initiatives: initiatives.len(),
        total_projects,
        initiatives,
    })
}

fn parse_initiative(node: &Value) -> Option<RoadmapInitiative> {
    let id = node.get("id")?.as_str()?.to_string();
    let name = node.get("name")?.as_str()?.to_string();
    let status = node
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();
    let target_date = node
        .get("targetDate")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let updated_at = node
        .get("updatedAt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let projects: Vec<RoadmapProject> = node
        .get("projects")
        .and_then(|p| p.get("nodes"))
        .and_then(|n| n.as_array())
        .map(|arr| arr.iter().filter_map(parse_project).collect())
        .unwrap_or_default();

    let avg_progress = if projects.is_empty() {
        0.0
    } else {
        projects.iter().map(|p| p.progress).sum::<f64>() / projects.len() as f64
    };

    Some(RoadmapInitiative {
        id,
        name,
        status,
        target_date,
        updated_at,
        projects,
        avg_progress,
    })
}

fn parse_project(node: &Value) -> Option<RoadmapProject> {
    let id = node.get("id")?.as_str()?.to_string();
    let name = node.get("name")?.as_str()?.to_string();
    let progress = node.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let state = node
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("backlog")
        .to_string();
    let target_date = node
        .get("targetDate")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let url = node
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(RoadmapProject {
        id,
        name,
        progress,
        state,
        target_date,
        url,
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
