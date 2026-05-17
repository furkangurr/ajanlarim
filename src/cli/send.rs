//! `agent-of-empires send` subcommand implementation
//!
//! AVK extension (FUR-4120): identifier `director`/`senior`/`worker`/`all`
//! keyword'leri AVK_AGENTS registry'sinden tier-filtered broadcast yapar.
//! Tier keyword AVK slug listesi ile çakışmaz (slug seti: koord/komuta/
//! mudur/code-1/code-2/merge/hata/codex/gemini-1/2/kimi-1/2/3).

use anyhow::{bail, Result};
use clap::Args;

use crate::avk_agents::resolve_tier_slugs;
use crate::session::{EnsureReadyError, EnsureReadyOutcome, Instance, Storage};

#[derive(Args)]
pub struct SendArgs {
    /// Session ID, title, or AVK tier keyword (director/senior/worker/all)
    identifier: String,

    /// Message to send to the agent
    message: String,

    /// Fail loud on dead/stopped sessions instead of auto-respawning. Default
    /// behavior is to revive the session so a `send` after a crash or stop
    /// just works; pass this for scripts that want the previous bail-out.
    #[arg(long = "no-revive")]
    no_revive: bool,
}

pub async fn run(profile: &str, args: SendArgs) -> Result<()> {
    let storage = Storage::new(profile)?;
    let (mut instances, _) = storage.load_with_groups()?;

    if args.message.trim().is_empty() {
        bail!("Message cannot be empty");
    }

    // AVK tier/all keyword broadcast vs. tekil session send ayrımı.
    let Some(avk_targets) = resolve_tier_slugs(args.identifier.as_str()) else {
        // Tekil session send (mevcut davranış)
        send_to_single(
            &mut instances,
            &args.identifier,
            &args.message,
            args.no_revive,
        )?;
        storage.save(&instances)?;
        return Ok(());
    };

    // Broadcast: AVK tier/all üzerinden multiple session send
    let total = avk_targets.len();
    let mut ok = 0usize;
    let mut failed: Vec<(String, String)> = Vec::new();

    println!(
        "Broadcasting to '{}' ({} sessions)...",
        args.identifier, total
    );
    for slug in &avk_targets {
        match send_to_single(&mut instances, slug, &args.message, args.no_revive) {
            Ok(()) => {
                ok += 1;
            }
            Err(e) => {
                eprintln!("  ✗ {slug}: {e}");
                failed.push(((*slug).to_string(), e.to_string()));
            }
        }
    }
    storage.save(&instances)?;

    println!("Broadcast complete: {ok}/{total} succeeded");
    if !failed.is_empty() {
        bail!("{} session(s) failed", failed.len());
    }
    Ok(())
}

/// Tek bir session'a mesaj gönder. Storage save dış arayan tarafından yapılır
/// (broadcast loop'unda her iterate save etmemek için ayrıştırıldı).
fn send_to_single(
    instances: &mut [Instance],
    identifier: &str,
    message: &str,
    no_revive: bool,
) -> Result<()> {
    let inst = super::resolve_session(identifier, instances)?;
    let session_id = inst.id.clone();
    let session_title = inst.title.clone();
    let tool = inst.tool.clone();

    if !no_revive {
        if let Some(target) = instances.iter_mut().find(|i| i.id == session_id) {
            match target.ensure_pane_ready() {
                Ok(EnsureReadyOutcome::Respawned) => {
                    eprintln!("  (respawned dead pane before send)");
                }
                Ok(EnsureReadyOutcome::Started) => {
                    eprintln!("  (started stopped session before send)");
                }
                Ok(EnsureReadyOutcome::AlreadyAlive) => {}
                Err(EnsureReadyError::Transient(status)) => {
                    bail!("Session is mid-lifecycle ({status:?}); cannot send right now")
                }
                Err(EnsureReadyError::CockpitMode) => {
                    bail!("Cockpit-mode sessions have no tmux pane; send is not supported")
                }
                Err(EnsureReadyError::Tmux(e)) => bail!("{}", e),
            }
        }
    }

    let tmux_session = crate::tmux::Session::new(&session_id, &session_title)?;
    if !tmux_session.exists() {
        bail!(
            "Session is not running. Start it first with: aoe session start {}",
            identifier
        );
    }

    let delay = crate::agents::send_keys_enter_delay(&tool);
    tmux_session.send_keys_with_delay(message, delay)?;

    if let Some(inst) = instances.iter_mut().find(|i| i.id == session_id) {
        inst.touch_last_accessed();
    }

    println!("Sent message to '{}'", session_title);
    Ok(())
}
