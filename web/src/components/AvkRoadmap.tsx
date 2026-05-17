/**
 * AVK roadmap widget — FUR-4165.
 *
 * `GET /api/avk/roadmap` Linear initiatives + projects progress. Her
 * initiative bir başlık altında alt projeler progress bar şeridi olarak
 * gösterilir. State renkli badge (started yeşil, planned sarı, completed
 * mavi, backlog/canceled gri).
 *
 * 5 dakika refresh — roadmap düşük frekanslı değişir, sustained polling
 * gereksiz.
 */

import { useEffect, useState } from "react";
import { fetchAvkRoadmap } from "../lib/api";
import type {
  RoadmapError,
  RoadmapInitiative,
  RoadmapProject,
  RoadmapResponse,
} from "../lib/types";

const REFRESH_INTERVAL_MS = 5 * 60_000;

const STATE_CLASS: Record<string, string> = {
  completed: "bg-status-running/20 text-status-running",
  started: "bg-status-waiting/20 text-status-waiting",
  planned: "bg-brand-500/20 text-brand-500",
  backlog: "bg-surface-700 text-text-muted",
  paused: "bg-surface-700 text-text-muted",
  canceled: "bg-status-error/10 text-status-error",
};

const STATE_LABEL: Record<string, string> = {
  completed: "Tamamlandı",
  started: "Devam",
  planned: "Planlı",
  backlog: "Birikim",
  paused: "Duraklatıldı",
  canceled: "İptal",
};

type RoadmapState =
  | { kind: "loading" }
  | { kind: "ready"; data: RoadmapResponse }
  | { kind: "not_configured"; message: string }
  | { kind: "error"; message: string };

function isErrorResponse(
  r: RoadmapResponse | RoadmapError | null,
): r is RoadmapError {
  return !!r && "error" in r && typeof (r as RoadmapError).error === "string";
}

export function AvkRoadmap() {
  const [state, setState] = useState<RoadmapState>({ kind: "loading" });

  useEffect(() => {
    let cancelled = false;
    async function load() {
      const res = await fetchAvkRoadmap();
      if (cancelled) return;
      if (!res) {
        setState({ kind: "error", message: "roadmap endpoint ulaşılamadı." });
        return;
      }
      if (isErrorResponse(res)) {
        if (res.kind === "not_configured") {
          setState({ kind: "not_configured", message: res.error });
        } else {
          setState({ kind: "error", message: res.error });
        }
        return;
      }
      setState({ kind: "ready", data: res });
    }
    load();
    const id = setInterval(load, REFRESH_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  if (state.kind === "loading") {
    return (
      <div>
        <Header />
        <p className="font-body text-[13px] text-text-muted">Yükleniyor…</p>
      </div>
    );
  }

  if (state.kind === "not_configured") {
    return (
      <div>
        <Header />
        <p className="font-body text-[13px] text-text-muted">
          Linear bağlantısı yapılandırılmamış — `LINEAR_API_KEY` env ile
          daemon'u yeniden başlatın.
        </p>
      </div>
    );
  }

  if (state.kind === "error") {
    return (
      <div>
        <Header />
        <p className="font-body text-[13px] text-status-error">
          roadmap hata: {state.message}
        </p>
      </div>
    );
  }

  const { initiatives, total_initiatives, total_projects } = state.data;

  return (
    <div>
      <Header
        initCount={total_initiatives}
        projCount={total_projects}
      />
      <div className="space-y-4">
        {initiatives.map((init) => (
          <InitiativeBlock key={init.id} init={init} />
        ))}
      </div>
    </div>
  );
}

function Header({
  initCount,
  projCount,
}: {
  initCount?: number;
  projCount?: number;
}) {
  return (
    <h3 className="font-mono text-sm uppercase tracking-widest text-text-muted mb-3">
      AVK Yol Haritası
      {typeof initCount === "number" && typeof projCount === "number" && (
        <span className="ml-2 normal-case tracking-normal text-text-dim text-[11px]">
          · {initCount} inisiyatif / {projCount} proje
        </span>
      )}
    </h3>
  );
}

function InitiativeBlock({ init }: { init: RoadmapInitiative }) {
  const pct = Math.round(init.avg_progress * 100);
  return (
    <section className="rounded border border-surface-700 bg-surface-800 p-3">
      <div className="flex items-baseline justify-between gap-2 mb-2 flex-wrap">
        <div className="flex items-center gap-2 min-w-0">
          <h4 className="font-body text-[14px] font-medium text-text-primary truncate">
            {init.name}
          </h4>
          <span className="font-mono text-[10px] uppercase tracking-wider px-1.5 py-0.5 rounded bg-surface-900 text-text-muted">
            {init.status}
          </span>
        </div>
        <div className="flex items-center gap-2 font-mono text-[11px] text-text-muted">
          <span className="text-text-secondary">{pct}%</span>
          <span>· {init.projects.length} proje</span>
          {init.target_date && (
            <span className="opacity-70">→ {init.target_date}</span>
          )}
        </div>
      </div>
      <ProgressBar pct={pct} />
      {init.projects.length > 0 && (
        <ul className="mt-2 space-y-1">
          {init.projects.map((proj) => (
            <ProjectRow key={proj.id} proj={proj} />
          ))}
        </ul>
      )}
    </section>
  );
}

function ProjectRow({ proj }: { proj: RoadmapProject }) {
  const pct = Math.round(proj.progress * 100);
  const stateClass = STATE_CLASS[proj.state] ?? STATE_CLASS.backlog;
  const stateLabel = STATE_LABEL[proj.state] ?? proj.state;
  return (
    <li className="flex items-center gap-2 px-2 py-1.5 rounded bg-surface-900 border border-surface-700">
      <span
        className={`font-mono text-[10px] uppercase tracking-wider px-1.5 py-0.5 rounded shrink-0 ${stateClass}`}
        title={proj.state}
      >
        {stateLabel}
      </span>
      <a
        href={proj.url || "#"}
        target="_blank"
        rel="noopener noreferrer"
        className="flex-1 min-w-0 font-body text-[12px] text-text-secondary hover:text-brand-500 transition-colors truncate"
        title={proj.name}
      >
        {proj.name}
      </a>
      <div className="hidden sm:flex items-center gap-2 shrink-0 w-32">
        <ProgressBar pct={pct} compact />
        <span className="font-mono text-[11px] text-text-muted w-9 text-right">
          {pct}%
        </span>
      </div>
      <span className="sm:hidden font-mono text-[11px] text-text-muted shrink-0">
        {pct}%
      </span>
    </li>
  );
}

function ProgressBar({ pct, compact = false }: { pct: number; compact?: boolean }) {
  const safe = Math.max(0, Math.min(100, pct));
  const color =
    safe >= 100
      ? "bg-status-running"
      : safe >= 50
        ? "bg-brand-500"
        : safe >= 20
          ? "bg-status-waiting"
          : "bg-surface-600";
  return (
    <div
      className={`w-full ${compact ? "h-1.5" : "h-2"} rounded-full bg-surface-900 overflow-hidden border border-surface-700`}
    >
      <div
        className={`h-full ${color} transition-all`}
        style={{ width: `${safe}%` }}
      />
    </div>
  );
}
