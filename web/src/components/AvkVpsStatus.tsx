/**
 * AVK VPS filo durum widget.
 *
 * `GET /api/avk/vps-status` → `{ hosts: AvkVpsHostEntry[] }` döner. Local
 * (primary) ilk, AOE_FLEET ile tanımlı uzak host'lar (runner vb.) peşinden.
 * Her host için ayrı kart: hostname/OS satırı + 6 metrik badge. Ulaşılamayan
 * host'ta error mesajı gösterilir. 30s refresh.
 */

import { useEffect, useState } from "react";
import { fetchAvkVpsStatus } from "../lib/api";
import type { AvkVpsHostEntry, AvkVpsStatusResponse } from "../lib/types";

const REFRESH_INTERVAL_MS = 30_000;

function formatUptime(sec: number | null): string {
  if (sec == null) return "—";
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

function formatKb(kb: number): string {
  if (kb < 1024) return `${kb}KB`;
  if (kb < 1024 * 1024) return `${(kb / 1024).toFixed(1)}MB`;
  if (kb < 1024 * 1024 * 1024) return `${(kb / 1024 / 1024).toFixed(1)}GB`;
  return `${(kb / 1024 / 1024 / 1024).toFixed(1)}TB`;
}

function pctAccent(pct: number): string {
  if (pct >= 90) return "text-status-error";
  if (pct >= 75) return "text-status-waiting";
  return "text-status-running";
}

function loadAccent(load: number, cpu: number | null): string {
  if (cpu == null) return "text-text-secondary";
  const ratio = load / cpu;
  if (ratio >= 1.5) return "text-status-error";
  if (ratio >= 0.8) return "text-status-waiting";
  return "text-status-running";
}

function roleLabel(role: string): string {
  switch (role) {
    case "primary":
      return "ana";
    case "runner":
      return "runner";
    default:
      return role;
  }
}

export function AvkVpsStatus() {
  const [fleet, setFleet] = useState<AvkVpsStatusResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [stale, setStale] = useState(false);
  const [selectedIdx, setSelectedIdx] = useState(0);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      const result = await fetchAvkVpsStatus();
      if (cancelled) return;
      if (result) {
        setFleet(result);
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
          VPS Durum
        </h3>
        <p className="font-body text-[13px] text-text-muted">Yükleniyor…</p>
      </div>
    );
  }

  if (!fleet || fleet.hosts.length === 0) {
    return (
      <div>
        <h3 className="font-mono text-sm uppercase tracking-widest text-text-muted mb-3">
          VPS Durum
        </h3>
        <p className="font-body text-[13px] text-status-error">
          VPS durum endpoint'i ulaşılamadı (`/api/avk/vps-status`).
        </p>
      </div>
    );
  }

  const activeIdx = Math.min(selectedIdx, fleet.hosts.length - 1);
  const activeHost = fleet.hosts[activeIdx];

  return (
    <div>
      <h3 className="font-mono text-sm uppercase tracking-widest text-text-muted mb-3">
        VPS Durum
        {stale && (
          <span className="ml-2 normal-case tracking-normal text-status-waiting text-[11px]">
            · yenilenemedi
          </span>
        )}
      </h3>
      {fleet.hosts.length > 1 && (
        <div
          role="tablist"
          aria-label="VPS host sekmeleri"
          className="flex gap-1 mb-3 border-b border-surface-700/60 overflow-x-auto"
        >
          {fleet.hosts.map((host, idx) => {
            const isActive = idx === activeIdx;
            return (
              <button
                key={`tab-${host.name}-${idx}`}
                role="tab"
                aria-selected={isActive}
                onClick={() => setSelectedIdx(idx)}
                className={`flex items-center gap-1.5 px-3 py-2 font-mono text-[12px] border-b-2 transition-colors whitespace-nowrap ${
                  isActive
                    ? "border-status-running text-text-primary bg-surface-800/40"
                    : "border-transparent text-text-secondary hover:text-text-primary hover:bg-surface-800/20"
                }`}
              >
                <span
                  className={`text-[10px] ${
                    host.ok ? "text-status-running" : "text-status-error"
                  }`}
                >
                  ●
                </span>
                <span>{host.hostname ?? host.name}</span>
                <span className="text-[10px] uppercase tracking-wider text-text-muted">
                  {roleLabel(host.role)}
                </span>
              </button>
            );
          })}
        </div>
      )}
      {activeHost && <HostCard host={activeHost} />}
    </div>
  );
}

function HostCard({ host }: { host: AvkVpsHostEntry }) {
  const load1 = host.load_avg?.[0];
  const load5 = host.load_avg?.[1];
  const load15 = host.load_avg?.[2];

  return (
    <div className="rounded border border-surface-700/60 bg-surface-900/40 p-3">
      <div className="flex items-center gap-2 mb-2">
        <span className="font-mono text-[12px] font-medium text-text-primary">
          {host.hostname ?? host.name}
        </span>
        <span className="font-mono text-[10px] uppercase tracking-wider px-1.5 py-0.5 rounded bg-surface-800 text-text-muted">
          {roleLabel(host.role)}
        </span>
        {host.ok ? (
          <span className="font-mono text-[10px] text-status-running">●</span>
        ) : (
          <span className="font-mono text-[10px] text-status-error">●</span>
        )}
      </div>

      {host.os || host.kernel ? (
        <p className="font-mono text-[11px] text-text-dim mb-2 truncate">
          {[host.os, host.kernel].filter(Boolean).join(" · ")}
        </p>
      ) : null}

      {!host.ok && host.error ? (
        <p className="font-body text-[12px] text-status-error">
          ulaşılamadı: {host.error}
        </p>
      ) : (
        <div className="grid grid-cols-2 lg:grid-cols-3 gap-2">
          <StatusBadge
            label="Süre"
            value={formatUptime(host.uptime_sec)}
            accent="text-text-secondary"
          />
          <StatusBadge
            label="CPU"
            value={host.cpu_count != null ? `${host.cpu_count} çekirdek` : "—"}
            accent="text-text-secondary"
          />
          <StatusBadge
            label="Yük 1dk"
            value={load1 != null ? load1.toFixed(2) : "—"}
            accent={load1 != null ? loadAccent(load1, host.cpu_count) : "text-text-secondary"}
          />
          <StatusBadge
            label="Yük 5/15dk"
            value={
              load5 != null && load15 != null
                ? `${load5.toFixed(2)} / ${load15.toFixed(2)}`
                : "—"
            }
            accent="text-text-secondary"
          />
          <StatusBadge
            label="Bellek"
            value={
              host.memory
                ? `%${host.memory.used_pct} · ${formatKb(host.memory.used_kb)}/${formatKb(host.memory.total_kb)}`
                : "—"
            }
            accent={host.memory ? pctAccent(host.memory.used_pct) : "text-text-secondary"}
          />
          <StatusBadge
            label={`Disk ${host.disk?.mount ?? ""}`.trim()}
            value={
              host.disk
                ? `%${host.disk.used_pct} · ${formatKb(host.disk.used_kb)}/${formatKb(host.disk.total_kb)}`
                : "—"
            }
            accent={host.disk ? pctAccent(host.disk.used_pct) : "text-text-secondary"}
          />
        </div>
      )}
    </div>
  );
}

function StatusBadge({
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
      <div className="font-mono text-[10px] uppercase tracking-wider text-text-muted truncate">
        {label}
      </div>
      <div className={`font-mono text-[13px] font-medium mt-0.5 ${accent} truncate`}>
        {value}
      </div>
    </div>
  );
}
