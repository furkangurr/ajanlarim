//! AVK Sentry alerts endpoint — FUR-4167.
//!
//! `GET /api/avk/sentry-alerts` Sentry REST API'sine (sentry.io/api/0)
//! `SENTRY_AUTH_TOKEN` + `SENTRY_ORG` env'i ile sorgu atar; son 24 saatte
//! çözülmemiş (unresolved) issue'ları döner. Dashboard widget'ı production
//! hata akışı özetini gösterir.
//!
//! ## Env
//!
//! - `SENTRY_AUTH_TOKEN` (zorunlu) — User Auth Token (Settings → Account → API)
//! - `SENTRY_ORG` (opsiyonel, default `avukata-danis`) — organization slug
//! - `SENTRY_PROJECT` (opsiyonel) — proje filtresi (verilmezse tüm proje'ler)
//!
//! Token yoksa 503 + `kind: not_configured`. Upstream hata 502.

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

const SENTRY_API_BASE: &str = "https://sentry.io/api/0";
const SENTRY_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_ORG: &str = "avukata-danis";
const QUERY_LIMIT: u32 = 12;
const STATS_PERIOD: &str = "24h";

#[derive(Debug, Serialize, Clone)]
pub struct SentryIssue {
    pub id: String,
    pub short_id: String,
    pub title: String,
    pub culprit: Option<String>,
    pub level: String,
    pub status: String,
    pub project: Option<String>,
    pub count: String,
    pub user_count: u64,
    pub first_seen: String,
    pub last_seen: String,
    pub permalink: String,
}

#[derive(Debug, Serialize)]
pub struct SentryAlertsResponse {
    pub org: String,
    pub project: Option<String>,
    pub period: &'static str,
    pub total: usize,
    pub issues: Vec<SentryIssue>,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'static str>,
}

pub async fn get_avk_sentry_alerts(State(_state): State<Arc<AppState>>) -> Response {
    let Some(token) = std::env::var("SENTRY_AUTH_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
    else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "SENTRY_AUTH_TOKEN env yapılandırılmamış",
            Some("not_configured"),
        );
    };

    let org = std::env::var("SENTRY_ORG").unwrap_or_else(|_| DEFAULT_ORG.to_string());
    let project = std::env::var("SENTRY_PROJECT")
        .ok()
        .filter(|v| !v.trim().is_empty());

    match fetch_issues(&token, &org, project.as_deref()).await {
        Ok(issues) => Json(SentryAlertsResponse {
            org,
            project,
            period: STATS_PERIOD,
            total: issues.len(),
            issues,
        })
        .into_response(),
        Err(e) => error_response(
            StatusCode::BAD_GATEWAY,
            &format!("Sentry fail: {e}"),
            Some("upstream_error"),
        ),
    }
}

async fn fetch_issues(
    token: &str,
    org: &str,
    project: Option<&str>,
) -> Result<Vec<SentryIssue>, String> {
    let mut url = format!(
        "{SENTRY_API_BASE}/organizations/{org}/issues/?query=is:unresolved&statsPeriod={STATS_PERIOD}&sort=date&limit={QUERY_LIMIT}",
    );
    if let Some(p) = project {
        url.push_str("&project=");
        url.push_str(p);
    }

    let client = reqwest::Client::builder()
        .timeout(SENTRY_TIMEOUT)
        .build()
        .map_err(|e| format!("reqwest build: {e}"))?;
    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| format!("Sentry unreachable: {e}"))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Sentry body read: {e}"))?;
    if !status.is_success() {
        return Err(format!("Sentry HTTP {status}: {}", truncate(&text, 200)));
    }

    let parsed: Value = serde_json::from_str(&text).map_err(|e| format!("Sentry parse: {e}"))?;
    let arr = parsed
        .as_array()
        .ok_or_else(|| "Sentry response not array".to_string())?;

    Ok(arr.iter().filter_map(parse_issue).collect())
}

fn parse_issue(node: &Value) -> Option<SentryIssue> {
    let id = node.get("id")?.as_str()?.to_string();
    let short_id = node
        .get("shortId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let title = node.get("title")?.as_str()?.to_string();
    let culprit = node
        .get("culprit")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let level = node
        .get("level")
        .and_then(|v| v.as_str())
        .unwrap_or("error")
        .to_string();
    let status = node
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unresolved")
        .to_string();
    let project = node
        .get("project")
        .and_then(|p| p.get("slug"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let count = node
        .get("count")
        .and_then(|v| v.as_str())
        .unwrap_or("0")
        .to_string();
    let user_count = node.get("userCount").and_then(|v| v.as_u64()).unwrap_or(0);
    let first_seen = node
        .get("firstSeen")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let last_seen = node
        .get("lastSeen")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let permalink = node
        .get("permalink")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(SentryIssue {
        id,
        short_id,
        title,
        culprit,
        level,
        status,
        project,
        count,
        user_count,
        first_seen,
        last_seen,
        permalink,
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
