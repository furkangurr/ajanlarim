//! AVK workflow ajan registry endpoint.
//!
//! FUR-3957 transplant Adım 6 — `/api/avk/agents` GET serve. Upstream
//! `list_agents` `/api/agents` AoE CLI binary tespiti döner (Claude/Cursor/
//! Codex); bu endpoint **bizim 13 workflow ajan** kaydını (Koord/Komuta/
//! Müdür/Code-1/2/Hata/Merge/Gemini-1/2/Kimi-1/2/3/Codex) JSON serve eder.
//!
//! Opsiyonel `?role=director|senior|worker` query filter. Geçersiz değer
//! 400 Bad Request döner (status, error JSON `{ "error": "..." }`).
//!
//! Atomic-Lock: FUR-3957 Adım 6 (Code-2 sole) — Koord karar 02:05Z + Adım 5
//! merge sonrası endpoint serve. Code-1 paralel F5.4 caller migration
//! (avukatadanis-online src/app/api/*) sustained.

use axum::{extract::Query, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::json;

use crate::avk_agents::{filter_by_role, AvkAgent, AvkAgentRole, AVK_AGENTS};

#[derive(Deserialize)]
pub struct AvkAgentsQuery {
    pub role: Option<String>,
}

/// GET `/api/avk/agents[?role=director|senior|worker]`
///
/// 13 AVK workflow ajan kayıt listesini JSON serve eder. `role` query
/// belirtilirse o tier filtrelenir; bilinmeyen değer 400 döner.
pub async fn list_avk_agents(Query(query): Query<AvkAgentsQuery>) -> impl IntoResponse {
    match query.role.as_deref() {
        None => {
            let agents: Vec<&AvkAgent> = AVK_AGENTS.iter().collect();
            Json(agents).into_response()
        }
        Some(raw) => {
            let role = match raw.to_ascii_lowercase().as_str() {
                "director" => AvkAgentRole::Director,
                "senior" => AvkAgentRole::Senior,
                "worker" => AvkAgentRole::Worker,
                other => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "error": format!(
                                "invalid role '{}': expected director|senior|worker",
                                other
                            )
                        })),
                    )
                        .into_response();
                }
            };
            let agents: Vec<&AvkAgent> = filter_by_role(role).collect();
            Json(agents).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::extract::Query;

    #[tokio::test]
    async fn list_returns_all_thirteen_when_no_filter() {
        let q = Query(AvkAgentsQuery { role: None });
        let response = list_avk_agents(q).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.len(), 13);
    }

    #[tokio::test]
    async fn list_filters_by_director() {
        let q = Query(AvkAgentsQuery {
            role: Some("director".to_string()),
        });
        let response = list_avk_agents(q).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.len(), 3);
        for agent in &parsed {
            assert_eq!(agent["role"], "director");
        }
    }

    #[tokio::test]
    async fn list_filters_by_senior() {
        let q = Query(AvkAgentsQuery {
            role: Some("senior".to_string()),
        });
        let response = list_avk_agents(q).await.into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.len(), 4);
    }

    #[tokio::test]
    async fn list_filters_by_worker() {
        let q = Query(AvkAgentsQuery {
            role: Some("worker".to_string()),
        });
        let response = list_avk_agents(q).await.into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.len(), 6);
    }

    #[tokio::test]
    async fn role_filter_case_insensitive() {
        let q = Query(AvkAgentsQuery {
            role: Some("DIRECTOR".to_string()),
        });
        let response = list_avk_agents(q).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn invalid_role_returns_bad_request() {
        let q = Query(AvkAgentsQuery {
            role: Some("admin".to_string()),
        });
        let response = list_avk_agents(q).await.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn agent_json_shape_has_required_fields() {
        let q = Query(AvkAgentsQuery { role: None });
        let response = list_avk_agents(q).await.into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        let first = &parsed[0];
        assert!(first.get("slug").is_some());
        assert!(first.get("label").is_some());
        assert!(first.get("role").is_some());
        assert!(first.get("tmux_target").is_some());
    }
}
