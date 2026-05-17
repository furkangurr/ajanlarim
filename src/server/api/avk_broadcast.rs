//! AVK tier broadcast endpoint — FUR-4121 Faz 3.
//!
//! `POST /api/avk/broadcast` mevcut tmux pane'lere doğrudan mesaj yollar.
//! `aoe send <tier> "<msg>"` CLI muadili; tier resolver
//! [`crate::avk_agents::resolve_tier_slugs`] shared, AVK_AGENTS registry
//! single source of truth.
//!
//! ## Tasarım kararı (Session vs. raw tmux)
//!
//! AVK ajanları AoE session olarak değil, doğrudan tmux pane'leri (avk-ofis
//! tmux session, idare/uretim/yardimcilar window'larında). Bu yüzden Session
//! struct + Storage resolver yerine `tmux send-keys -t <target>` raw çağrı
//! kullanılır. Pane var olup olmadığı `tmux list-panes -F` ile pre-check.
//!
//! ## Güvenlik
//!
//! Message validation:
//!   - Boş mesaj reject (400)
//!   - 8KB cap (paste-buffer threshold + safety margin)
//!   - Bilinmeyen tier reject (404)
//!
//! Multi-line / 2KB+ mesaj için tmux paste-buffer path (bracketed paste)
//! kullanılır — agent CLI'lar (Claude, Codex, Gemini, Kimi) DECSET 2004
//! ingest eder, satır satır submit yerine atomic paste alır.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::AppState;
use crate::avk_agents::{find_by_slug, resolve_tier_slugs};

const MAX_MESSAGE_BYTES: usize = 8192;
const PASTE_BUFFER_THRESHOLD: usize = 2048;

#[derive(Deserialize)]
pub struct BroadcastRequest {
    /// Tier keyword: `director` / `senior` / `worker` / `all`.
    pub tier: String,
    /// Gönderilecek mesaj (8KB cap).
    pub message: String,
}

#[derive(Serialize)]
pub struct BroadcastResult {
    pub slug: String,
    pub target: String,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct BroadcastResponse {
    pub tier: String,
    pub total: usize,
    pub ok: usize,
    pub failed: usize,
    pub results: Vec<BroadcastResult>,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

pub async fn broadcast_avk(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<BroadcastRequest>,
) -> Response {
    let message = req.message.trim();
    if message.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "message cannot be empty");
    }
    if message.len() > MAX_MESSAGE_BYTES {
        return error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            &format!(
                "message too long ({}B > cap {}B)",
                message.len(),
                MAX_MESSAGE_BYTES
            ),
        );
    }

    let Some(slugs) = resolve_tier_slugs(req.tier.as_str()) else {
        return error_response(
            StatusCode::NOT_FOUND,
            &format!(
                "unknown tier '{}' (expected: director / senior / worker / all)",
                req.tier
            ),
        );
    };

    let mut results = Vec::with_capacity(slugs.len());
    let mut ok = 0usize;
    let mut failed = 0usize;

    for slug in &slugs {
        let agent = match find_by_slug(slug) {
            Some(a) => a,
            None => {
                results.push(BroadcastResult {
                    slug: (*slug).to_string(),
                    target: String::new(),
                    ok: false,
                    error: Some("slug not found in AVK_AGENTS registry".into()),
                });
                failed += 1;
                continue;
            }
        };

        match send_to_pane(agent.tmux_target, message) {
            Ok(()) => {
                ok += 1;
                results.push(BroadcastResult {
                    slug: (*slug).to_string(),
                    target: agent.tmux_target.to_string(),
                    ok: true,
                    error: None,
                });
            }
            Err(e) => {
                failed += 1;
                results.push(BroadcastResult {
                    slug: (*slug).to_string(),
                    target: agent.tmux_target.to_string(),
                    ok: false,
                    error: Some(e),
                });
            }
        }
    }

    Json(BroadcastResponse {
        tier: req.tier,
        total: slugs.len(),
        ok,
        failed,
        results,
    })
    .into_response()
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

/// Tek bir tmux pane'e mesaj gönder + Enter submit.
///
/// Pane var olup olmadığı `list-panes` ile pre-check (yanlış registry +
/// tmux drift yakalama). Multi-line / 2KB+ mesajlar paste-buffer üzerinden
/// bracketed paste, kısa tek satır mesajlar `send-keys -l --`.
fn send_to_pane(target: &str, message: &str) -> Result<(), String> {
    if !pane_exists(target)? {
        return Err(format!("tmux pane not found: {target}"));
    }

    let use_paste_buffer = message.len() >= PASTE_BUFFER_THRESHOLD || message.contains('\n');
    if use_paste_buffer {
        send_via_paste_buffer(target, message)?;
    } else {
        run_tmux(&["send-keys", "-t", target, "-l", "--", message])?;
    }

    run_tmux(&["send-keys", "-t", target, "Enter"])?;
    Ok(())
}

fn pane_exists(target: &str) -> Result<bool, String> {
    let output = Command::new("tmux")
        .args([
            "list-panes",
            "-a",
            "-F",
            "#{session_name}:#{window_name}.#{pane_index}",
        ])
        .output()
        .map_err(|e| format!("tmux list-panes spawn failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "tmux list-panes failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().any(|line| line.trim() == target))
}

fn run_tmux(args: &[&str]) -> Result<(), String> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .map_err(|e| format!("tmux spawn failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "tmux {} failed: {}",
            args.first().copied().unwrap_or(""),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn send_via_paste_buffer(target: &str, text: &str) -> Result<(), String> {
    static SEND_COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = SEND_COUNTER.fetch_add(1, Ordering::Relaxed);
    let buf_name = format!("aoe-avk-broadcast-{}-{}", std::process::id(), seq);

    let mut child = Command::new("tmux")
        .args(["load-buffer", "-b", &buf_name, "-"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("tmux load-buffer spawn failed: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("tmux load-buffer stdin write failed: {e}"))?;
    }
    let status = child
        .wait()
        .map_err(|e| format!("tmux load-buffer wait failed: {e}"))?;
    if !status.success() {
        return Err(format!(
            "tmux load-buffer exited non-zero (code={:?})",
            status.code()
        ));
    }

    let output = Command::new("tmux")
        .args(["paste-buffer", "-d", "-p", "-b", &buf_name, "-t", target])
        .output()
        .map_err(|e| format!("tmux paste-buffer spawn failed: {e}"))?;
    if !output.status.success() {
        let _ = Command::new("tmux")
            .args(["delete-buffer", "-b", &buf_name])
            .output();
        return Err(format!(
            "tmux paste-buffer failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}
