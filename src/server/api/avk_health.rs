//! AVK system health endpoint — FUR-4157.
//!
//! `GET /api/avk/health` AoE binary + AVK pane registry için kompakt sağlık
//! özeti döner. UI Dashboard AVK Komuta Paneli kart şeridinde live badge
//! olarak gösterir (tmux + canlı ajan oranı + version + uptime).
//!
//! Uptime bu modül ilk yüklendiğinde (`std::sync::OnceLock<Instant>`)
//! başlar; AoE serve daemon ömrünün proxy'sidir. Daemon restart sonrası
//! sıfırlanır.

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::process::Command;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use super::avk_broadcast::resolve_runtime_target;
use super::AppState;
use crate::avk_agents::AVK_AGENTS;

static START_AT: OnceLock<Instant> = OnceLock::new();

#[derive(Serialize)]
pub struct AvkHealthResponse {
    /// AoE binary semver (Cargo.toml).
    pub version: &'static str,
    /// Endpoint ilk çağrısından bu yana geçen saniye (daemon uptime proxy).
    pub uptime_sec: u64,
    /// `tmux list-sessions` exit 0 dönüyor mu (tmux sunucu çalışıyor).
    pub tmux_ok: bool,
    /// Registry'deki AVK ajan sayısı (AVK_AGENTS.len() = 13).
    pub agent_count: usize,
    /// Bunlardan kaçının runtime tmux pane'i çözüldü (canlı).
    pub agent_alive: usize,
}

pub async fn get_avk_health(State(_state): State<Arc<AppState>>) -> Response {
    let start = START_AT.get_or_init(Instant::now);
    let uptime_sec = start.elapsed().as_secs();
    let tmux_ok = check_tmux();
    let agent_alive = AVK_AGENTS
        .iter()
        .filter(|a| resolve_runtime_target(a.slug).is_some())
        .count();

    Json(AvkHealthResponse {
        version: env!("CARGO_PKG_VERSION"),
        uptime_sec,
        tmux_ok,
        agent_count: AVK_AGENTS.len(),
        agent_alive,
    })
    .into_response()
}

fn check_tmux() -> bool {
    Command::new("tmux")
        .arg("list-sessions")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
