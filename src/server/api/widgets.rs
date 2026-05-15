//! Linear + Sentry summary widget endpoints (avk-suite custom adapt).
//!
//! Two REST GET endpoints under `/api/widgets/`:
//!   - `/linear/summary` — In Progress + Backlog + 7d Done counts + 5 recent issue
//!   - `/sentry/summary` — Last 24h unresolved issue count + 5 recent
//!
//! Each backed by 60-second in-process cache to absorb dashboard refresh bursts
//! without hitting Linear/Sentry rate limits. Env-driven (LINEAR_API_KEY,
//! SENTRY_AUTH_TOKEN, SENTRY_ORG, SENTRY_PROJECT_SLUG); missing env returns 500
//! with a precise error (no silent fallback).
//!
//! Equivalent port of the avk Sub-C Next.js routes (avk PR ajan-sistemi#521).

use std::env;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::AppState;

const CACHE_TTL: Duration = Duration::from_secs(60);
const LINEAR_TEAM_ID: &str = "5669ce66-e92d-4a8a-96c3-49be9a68204f";
const LINEAR_GRAPHQL: &str = "https://api.linear.app/graphql";

#[derive(Clone, Serialize)]
pub struct LinearCount {
    pub count: usize,
    pub has_more: bool,
}

#[derive(Clone, Serialize)]
pub struct LinearIssue {
    pub identifier: String,
    pub title: String,
    pub state: String,
    pub url: String,
    pub updated_at: String,
}

#[derive(Clone, Serialize)]
pub struct LinearSummary {
    pub in_progress: LinearCount,
    pub backlog: LinearCount,
    pub done7d: LinearCount,
    pub recent: Vec<LinearIssue>,
    pub fetched_at: String,
}

#[derive(Clone, Serialize)]
pub struct SentryIssue {
    pub id: String,
    pub short_id: String,
    pub title: String,
    pub culprit: String,
    pub count: String,
    pub permalink: String,
    pub last_seen: String,
}

#[derive(Clone, Serialize)]
pub struct SentrySummary {
    pub total_issues: usize,
    pub unresolved: usize,
    pub recent: Vec<SentryIssue>,
    pub fetched_at: String,
}

struct Cached<T> {
    data: T,
    expires_at: Instant,
}

#[derive(Default)]
pub struct WidgetCache {
    linear: Mutex<Option<Cached<LinearSummary>>>,
    sentry: Mutex<Option<Cached<SentrySummary>>>,
}

impl WidgetCache {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_linear(&self) -> Option<LinearSummary> {
        let guard = self.linear.lock().ok()?;
        let entry = guard.as_ref()?;
        if entry.expires_at > Instant::now() {
            Some(entry.data.clone())
        } else {
            None
        }
    }

    fn set_linear(&self, data: LinearSummary) {
        if let Ok(mut guard) = self.linear.lock() {
            *guard = Some(Cached {
                data,
                expires_at: Instant::now() + CACHE_TTL,
            });
        }
    }

    fn get_sentry(&self) -> Option<SentrySummary> {
        let guard = self.sentry.lock().ok()?;
        let entry = guard.as_ref()?;
        if entry.expires_at > Instant::now() {
            Some(entry.data.clone())
        } else {
            None
        }
    }

    fn set_sentry(&self, data: SentrySummary) {
        if let Ok(mut guard) = self.sentry.lock() {
            *guard = Some(Cached {
                data,
                expires_at: Instant::now() + CACHE_TTL,
            });
        }
    }
}

#[derive(Deserialize)]
struct LinearGraphResp {
    data: Option<LinearGraphData>,
    errors: Option<Vec<LinearGraphError>>,
}

#[derive(Deserialize)]
struct LinearGraphError {
    message: String,
}

#[derive(Deserialize)]
struct LinearGraphData {
    #[serde(rename = "inProgress")]
    in_progress: LinearConn,
    backlog: LinearConn,
    #[serde(rename = "done7d")]
    done7d: LinearConn,
    recent: LinearRecentConn,
}

#[derive(Deserialize)]
struct LinearConn {
    nodes: Vec<Value>,
    #[serde(rename = "pageInfo")]
    page_info: LinearPageInfo,
}

#[derive(Deserialize)]
struct LinearPageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
}

#[derive(Deserialize)]
struct LinearRecentConn {
    nodes: Vec<LinearRecentNode>,
}

#[derive(Deserialize)]
struct LinearRecentNode {
    identifier: String,
    title: String,
    url: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    state: LinearStateRef,
}

#[derive(Deserialize)]
struct LinearStateRef {
    name: String,
}

const LINEAR_QUERY: &str = r#"
query Summary($teamId: ID!, $since: DateTimeOrDuration!) {
  inProgress: issues(filter: {team: {id: {eq: $teamId}}, state: {type: {eq: "started"}}}, first: 250) {
    nodes { id }
    pageInfo { hasNextPage }
  }
  backlog: issues(filter: {team: {id: {eq: $teamId}}, state: {type: {in: ["backlog", "unstarted"]}}}, first: 250) {
    nodes { id }
    pageInfo { hasNextPage }
  }
  done7d: issues(filter: {team: {id: {eq: $teamId}}, state: {type: {eq: "completed"}}, completedAt: {gte: $since}}, first: 250) {
    nodes { id }
    pageInfo { hasNextPage }
  }
  recent: issues(filter: {team: {id: {eq: $teamId}}}, orderBy: updatedAt, first: 5) {
    nodes {
      identifier
      title
      url
      updatedAt
      state { name }
    }
  }
}
"#;

async fn fetch_linear(api_key: &str) -> Result<LinearSummary, String> {
    let since = chrono::Utc::now() - chrono::Duration::days(7);
    let since_iso = since.to_rfc3339();
    let body = serde_json::json!({
        "query": LINEAR_QUERY,
        "variables": { "teamId": LINEAR_TEAM_ID, "since": since_iso }
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(LINEAR_GRAPHQL)
        .header("Authorization", api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Linear request error: {e}"))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Linear body read error: {e}"))?;

    if !status.is_success() {
        return Err(format!("Linear API {status}: {text}"));
    }

    let parsed: LinearGraphResp = serde_json::from_str(&text)
        .map_err(|e| format!("Linear JSON parse error: {e} | body: {text}"))?;

    if let Some(errs) = parsed.errors {
        let msg = errs
            .into_iter()
            .map(|e| e.message)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("Linear GraphQL error: {msg}"));
    }

    let data = parsed.data.ok_or_else(|| "Linear data missing".to_string())?;

    Ok(LinearSummary {
        in_progress: LinearCount {
            count: data.in_progress.nodes.len(),
            has_more: data.in_progress.page_info.has_next_page,
        },
        backlog: LinearCount {
            count: data.backlog.nodes.len(),
            has_more: data.backlog.page_info.has_next_page,
        },
        done7d: LinearCount {
            count: data.done7d.nodes.len(),
            has_more: data.done7d.page_info.has_next_page,
        },
        recent: data
            .recent
            .nodes
            .into_iter()
            .map(|n| LinearIssue {
                identifier: n.identifier,
                title: n.title,
                state: n.state.name,
                url: n.url,
                updated_at: n.updated_at,
            })
            .collect(),
        fetched_at: chrono::Utc::now().to_rfc3339(),
    })
}

#[derive(Deserialize)]
struct SentryRaw {
    id: String,
    #[serde(rename = "shortId")]
    short_id: String,
    title: String,
    culprit: String,
    count: String,
    permalink: String,
    #[serde(rename = "lastSeen")]
    last_seen: String,
    status: String,
}

/// Sentry slug/org validation: alphanumeric + dash/underscore/dot. Sentry
/// slugs by convention are lowercase kebab; we reject anything outside that
/// alphabet so the URL stays well-formed without a percent-encoder.
fn valid_sentry_slug(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

async fn fetch_sentry(
    auth_token: &str,
    org: &str,
    project_slug: &str,
) -> Result<SentrySummary, String> {
    if !valid_sentry_slug(org) || !valid_sentry_slug(project_slug) {
        return Err(
            "Sentry org/project_slug geçersiz karakter içeriyor (alphanumeric + -_. izinli)"
                .to_string(),
        );
    }

    let url = format!(
        "https://sentry.io/api/0/projects/{org}/{project_slug}/issues/?statsPeriod=24h&limit=25&sort=date&query=is:unresolved"
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {auth_token}"))
        .send()
        .await
        .map_err(|e| format!("Sentry request error: {e}"))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Sentry body read error: {e}"))?;

    if !status.is_success() {
        return Err(format!("Sentry API {status}: {text}"));
    }

    let raw: Vec<SentryRaw> = serde_json::from_str(&text)
        .map_err(|e| format!("Sentry JSON parse error: {e}"))?;

    let unresolved: Vec<&SentryRaw> = raw.iter().filter(|r| r.status == "unresolved").collect();
    let total = raw.len();
    let unresolved_count = unresolved.len();

    Ok(SentrySummary {
        total_issues: total,
        unresolved: unresolved_count,
        recent: unresolved
            .into_iter()
            .take(5)
            .map(|r| SentryIssue {
                id: r.id.clone(),
                short_id: r.short_id.clone(),
                title: r.title.clone(),
                culprit: r.culprit.clone(),
                count: r.count.clone(),
                permalink: r.permalink.clone(),
                last_seen: r.last_seen.clone(),
            })
            .collect(),
        fetched_at: chrono::Utc::now().to_rfc3339(),
    })
}

pub async fn get_linear_summary(
    State(state): State<Arc<AppState>>,
) -> Result<Json<LinearSummary>, (StatusCode, String)> {
    let api_key = env::var("LINEAR_API_KEY").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "LINEAR_API_KEY env eksik".to_string(),
        )
    })?;

    if let Some(cached) = state.widget_cache.get_linear() {
        return Ok(Json(cached));
    }

    match fetch_linear(&api_key).await {
        Ok(data) => {
            state.widget_cache.set_linear(data.clone());
            Ok(Json(data))
        }
        Err(msg) => Err((StatusCode::BAD_GATEWAY, msg)),
    }
}

pub async fn get_sentry_summary(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SentrySummary>, (StatusCode, String)> {
    let auth = env::var("SENTRY_AUTH_TOKEN").ok();
    let org = env::var("SENTRY_ORG").ok();
    let project = env::var("SENTRY_PROJECT_SLUG").ok();

    let (auth, org, project) = match (auth, org, project) {
        (Some(a), Some(o), Some(p)) if !a.is_empty() && !o.is_empty() && !p.is_empty() => {
            (a, o, p)
        }
        _ => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Sentry env eksik (SENTRY_AUTH_TOKEN + SENTRY_ORG + SENTRY_PROJECT_SLUG gerekli)"
                    .to_string(),
            ));
        }
    };

    if let Some(cached) = state.widget_cache.get_sentry() {
        return Ok(Json(cached));
    }

    match fetch_sentry(&auth, &org, &project).await {
        Ok(data) => {
            state.widget_cache.set_sentry(data.clone());
            Ok(Json(data))
        }
        Err(msg) => Err((StatusCode::BAD_GATEWAY, msg)),
    }
}
