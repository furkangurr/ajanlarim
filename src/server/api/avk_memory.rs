//! AVK memory recall feed endpoint (FUR-4118).
//!
//! `GET /api/avk/memory-recall[?role=koord&hours=24]` — agentmemory MCP'ye
//! HTTP/JSON-RPC proxy aşaması bekleniyor (VPS-side endpoint research
//! gerek); şimdilik mock data ile UI contract validate edilir.
//!
//! Mock entries gerçek son 24 saat patrol özet'inden örnek alır:
//! FUR-3957 transplant tamamlandı, FUR-4120 tier broadcast eklendi,
//! P0 prod incident RESOLVED vs. Furkan'a gerçek-feel demo verir.

use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AvkMemoryQuery {
    pub role: Option<String>,
    pub hours: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryTier {
    Core,
    Working,
    Archival,
}

#[derive(Debug, Serialize)]
pub struct MemoryEntry {
    pub id: &'static str,
    pub title: &'static str,
    pub tier: MemoryTier,
    pub role: &'static str,
    pub tags: &'static [&'static str],
    pub content_preview: &'static str,
    pub created_at: &'static str,
}

/// Mock recall feed — son 24 saat canon kayıtlar.
///
/// Gerçek implementation agentmemory MCP'ye reqwest HTTP/JSON-RPC ile
/// bağlanır. VPS-side endpoint spec'i hazır olunca bu fonksiyon mock
/// yerine gerçek query çalıştırır.
const MOCK_FEED: &[MemoryEntry] = &[
    MemoryEntry {
        id: "mem-001",
        title: "FUR-3957 transplant TAMAMLANDI — Adım 5-8 zinciri",
        tier: MemoryTier::Core,
        role: "omni",
        tags: &["ajanlarim", "aoe", "transplant", "tamamlandı"],
        content_preview:
            "PR #4 admin merge başarılı. AvkAgentsGrid widget Dashboard'a mount edildi, \
             /api/avk/agents endpoint LIVE, avk_agents.rs registry main'de. 3 dormant \
             branch (F6/F7/F8) silindi, 1 obsolete (avk/kimi-adapt-level1) silindi. \
             Final state temiz: sadece main.",
        created_at: "2026-05-17T13:10:00Z",
    },
    MemoryEntry {
        id: "mem-002",
        title: "FUR-4120 tier broadcast PR #6 MERGED",
        tier: MemoryTier::Core,
        role: "omni",
        tags: &["inject-bridge", "tier-broadcast", "send"],
        content_preview:
            "aoe send director|senior|worker|all <mesaj> — AVK_AGENTS registry'sinden \
             tier-filtered multiple session send. Test: director 3/3 ✓, senior 4/4 ✓. \
             Backward-compat slug positional korundu. Atomic single-PR.",
        created_at: "2026-05-17T13:54:38Z",
    },
    MemoryEntry {
        id: "mem-003",
        title: "FUR-4117 — 13 ajan AoE session lifecycle lokal",
        tier: MemoryTier::Working,
        role: "omni",
        tags: &["aoe", "session-lifecycle", "lokal-first"],
        content_preview:
            "13 ajan (koord/komuta/mudur/code-1/2/merge/hata/codex/gemini-1/2/kimi-1/2/3) \
             AoE'ye eklendi (aoe add). --launch yok, kotaya değmedi. Brew install aoe \
             1.7.0 + cargo build --features serve (4m 14s). Mevcut tmux setup paralel.",
        created_at: "2026-05-17T13:38:00Z",
    },
    MemoryEntry {
        id: "mem-004",
        title: "P0 PROD INCIDENT RESOLVED — FUR-4073 8 tezahür",
        tier: MemoryTier::Core,
        role: "hata",
        tags: &["p0", "prod", "resolved", "strangler-cluster"],
        content_preview:
            "5h 30m sustained Strangler refactor cluster (FAZ-2J → FAZ-2T). 8 tezahür, \
             5 fix PR (#7600/7602/7609/7612/7613/7621/7625). Synergy: Omni patrol detect \
             + Hata Ajanı pickup loop 5-30dk turnaround. www.avukatadanis.com 200 OK ✓.",
        created_at: "2026-05-17T07:05:00Z",
    },
    MemoryEntry {
        id: "mem-005",
        title: "SEO audit revize — Furkan canon domain migration",
        tier: MemoryTier::Working,
        role: "omni",
        tags: &["seo", "fur-4072", "verify-before-claim"],
        content_preview:
            "Furkan 'domaini avukatadanis.com taşıdık' tek cümleyle düzeltti. \
             Önceki iddia 'ENV eksik, fallback eski domain' YANLIŞ — verify edilmiş \
             değil. Karpathy 51 #31 ders. Doğru tanı: GSC'de yeni .com property \
             eklenmemiş. Linear FUR-4072 P1 handover Furkan Google hesabı.",
        created_at: "2026-05-17T02:11:20Z",
    },
    MemoryEntry {
        id: "mem-006",
        title: "Karpathy 51 #137 — Mekanik > niyet, FUR-4068 guard yetersiz",
        tier: MemoryTier::Archival,
        role: "koord",
        tags: &["karpathy", "mekanik-niyet", "pre-commit-hook"],
        content_preview:
            "FUR-4068 Strangler rename pre-commit guard (5-pattern grep) MERGED ama 1 saat \
             içinde 2 yeni cluster tezahürü (FAZ-2Q + 2T). Guard caller-side path tarıyor, \
             target-side relative import resolve test eksik. Mekanik garanti tanım \
             yetersizliği etkisiz. Önlem aday: pre-PR lokal build zorunluluğu canon.",
        created_at: "2026-05-17T06:42:54Z",
    },
    MemoryEntry {
        id: "mem-007",
        title: "tmux Yardimcilar window rebuild — kalıcı kök çözüm",
        tier: MemoryTier::Working,
        role: "omni",
        tags: &["tmux", "drift", "kalici-cozum", "karpathy-10-1"],
        content_preview:
            "Furkan canon 'birkaç sefer ayarladık, yeniden başlatınca karışıyor yardımcılar'. \
             Manuel yama yerine idempotent rebuild script (Karpathy §10.1). \
             avk-office:2 6 pane 2col×3row (G1|G2 / K1|K2 / K3|Codex). \
             ajan-sistemi PR #674 merged. Restart sonrası tek komutla layout.",
        created_at: "2026-05-17T11:55:00Z",
    },
];

/// GET `/api/avk/memory-recall[?role=...&hours=...]`
///
/// Mock implementation: filter sadece `role` parametresiyle çalışır
/// (hours şu an yok-sayılır; gerçek MCP query implementation eklendiğinde
/// `created_at` aralık filter aktif). Bilinmeyen role boş array döner.
pub async fn list_avk_memory_recall(Query(query): Query<AvkMemoryQuery>) -> impl IntoResponse {
    let filtered: Vec<&MemoryEntry> = MOCK_FEED
        .iter()
        .filter(|entry| match query.role.as_deref() {
            Some(role) => entry.role == role,
            None => true,
        })
        .collect();
    Json(filtered).into_response()
}
