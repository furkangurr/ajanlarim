/**
 * AVK sistem sağlık widget — FUR-4157.
 *
 * `GET /api/avk/health` çağırır; AoE binary version + daemon uptime +
 * tmux durum + canlı ajan oranı (X/13) kompakt badge satırı render eder.
 * 30s refresh interval — AvkAgentsGrid ile aynı kadans.
 *
 * `uptime_sec` server module ilk yüklendiğinden bu yana (daemon ömrü
 * proxy). Restart sonrası sıfırlanır.
 */

import { useEffect, useState } from "react";
import { fetchAvkHealth } from "../lib/api";
import type { AvkHealthResponse } from "../lib/types";

const REFRESH_INTERVAL_MS = 30_000;

function formatUptime(sec: number): string {
  if (sec < 60) return `${sec}sn`;
  if (sec < 3600) return `${Math.floor(sec / 60)}dk`;
  if (sec < 86_400) {
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    return m > 0 ? `${h}sa ${m}dk` : `${h}sa`;
  }
  const d = Math.floor(sec / 86_400);
  const h = Math.floor((sec % 86_400) / 3600);
  return h > 0 ? `${d}g ${h}sa` : `${d}g`;
}

export function AvkSystemHealth() {
  const [health, setHealth] = useState<AvkHealthResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [stale, setStale] = useState(false);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      const result = await fetchAvkHealth();
      if (cancelled) return;
      if (result) {
        setHealth(result);
        setStale(false);
      } else {
        setStale(true);
      }
      setLoading(false);
    }
    load();
    const id = setInterval(load, REFRESH_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  if (loading) {
    return (
      <div>
        <h3 className="font-mono text-sm uppercase tracking-widest text-text-muted mb-3">
          AVK Sağlık
        </h3>
        <p className="font-body text-[13px] text-text-muted">Yükleniyor…</p>
      </div>
    );
  }

  if (!health) {
    return (
      <div>
        <h3 className="font-mono text-sm uppercase tracking-widest text-text-muted mb-3">
          AVK Sağlık
        </h3>
        <p className="font-body text-[13px] text-status-error">
          Sağlık endpoint'i ulaşılamadı (`/api/avk/health`).
        </p>
      </div>
    );
  }

  const aliveRatio = health.agent_count > 0
    ? health.agent_alive / health.agent_count
    : 0;
  const aliveClass =
    aliveRatio >= 0.8
      ? "text-status-running"
      : aliveRatio >= 0.4
        ? "text-status-waiting"
        : "text-status-error";

  return (
    <div>
      <h3 className="font-mono text-sm uppercase tracking-widest text-text-muted mb-3">
        AVK Sağlık
        {stale && (
          <span className="ml-2 normal-case tracking-normal text-status-waiting text-[11px]">
            · yenilenemedi
          </span>
        )}
      </h3>
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-2">
        <HealthBadge
          label="Sürüm"
          value={`v${health.version}`}
          accent="text-text-secondary"
        />
        <HealthBadge
          label="Süre"
          value={formatUptime(health.uptime_sec)}
          accent="text-text-secondary"
        />
        <HealthBadge
          label="tmux"
          value={health.tmux_ok ? "Bağlı" : "Yok"}
          accent={health.tmux_ok ? "text-status-running" : "text-status-error"}
        />
        <HealthBadge
          label="Ajan"
          value={`${health.agent_alive}/${health.agent_count}`}
          accent={aliveClass}
        />
      </div>
    </div>
  );
}

function HealthBadge({
  label,
  value,
  accent,
}: {
  label: string;
  value: string;
  accent: string;
}) {
  return (
    <div className="rounded border border-surface-700 bg-surface-800 px-3 py-2">
      <div className="font-mono text-[10px] uppercase tracking-wider text-text-muted">
        {label}
      </div>
      <div className={`font-mono text-[14px] font-medium mt-0.5 ${accent}`}>
        {value}
      </div>
    </div>
  );
}
