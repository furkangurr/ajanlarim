/**
 * AVK Linear iş kuyruğu widget — FUR-4160.
 *
 * `GET /api/avk/linear-queue` çağırır; Aktif (started) + Sıradaki (unstarted)
 * iki bölüm halinde ilk 6 issue gösterir. Priority badge (0-4: No/Urgent/High/Medium/Low),
 * assignee, team key, relative time.
 *
 * 60s refresh interval — Linear API rate limit (1500 req/15dk team) cömert,
 * dashboard sustained refresh için yeterli kadans.
 *
 * Backend ENV (`LINEAR_API_KEY`) yoksa "yapılandırma eksik" mesajı; upstream
 * hatası "Linear ulaşılamadı" notu. Mock yok (gerçek olmayınca anlam taşımıyor).
 */

import { useEffect, useState } from "react";
import { fetchAvkLinearQueue } from "../lib/api";
import type {
  LinearIssue,
  LinearQueueError,
  LinearQueueResponse,
} from "../lib/types";

const REFRESH_INTERVAL_MS = 60_000;
const MAX_PER_SECTION = 6;

const PRIORITY_LABEL: Record<number, string> = {
  0: "Yok",
  1: "Acil",
  2: "Yüksek",
  3: "Orta",
  4: "Düşük",
};

const PRIORITY_CLASS: Record<number, string> = {
  1: "bg-status-error/15 text-status-error border-status-error/30",
  2: "bg-status-waiting/15 text-status-waiting border-status-waiting/30",
  3: "bg-surface-700 text-text-secondary border-surface-600",
  4: "bg-surface-800 text-text-muted border-surface-700",
  0: "bg-surface-800 text-text-dim border-surface-700",
};

function formatRelativeTime(iso: string): string {
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return iso;
  const now = Date.now();
  const diffMin = Math.floor((now - then) / 60_000);
  if (diffMin < 1) return "az önce";
  if (diffMin < 60) return `${diffMin}dk önce`;
  const diffHours = Math.floor(diffMin / 60);
  if (diffHours < 24) return `${diffHours}sa önce`;
  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays}g önce`;
}

type QueueState =
  | { kind: "loading" }
  | { kind: "ready"; data: LinearQueueResponse }
  | { kind: "not_configured"; message: string }
  | { kind: "error"; message: string };

function isErrorResponse(
  r: LinearQueueResponse | LinearQueueError | null,
): r is LinearQueueError {
  return !!r && "error" in r && typeof (r as LinearQueueError).error === "string";
}

export function AvkLinearQueue() {
  const [state, setState] = useState<QueueState>({ kind: "loading" });

  useEffect(() => {
    let cancelled = false;
    async function load() {
      const res = await fetchAvkLinearQueue();
      if (cancelled) return;
      if (!res) {
        setState({ kind: "error", message: "Linear endpoint ulaşılamadı." });
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
          Linear bağlantısı yapılandırılmamış —{" "}
          <code className="font-mono text-text-secondary">LINEAR_API_KEY</code>{" "}
          env ile <code className="font-mono text-text-secondary">aoe serve</code>{" "}
          yeniden başlatın.
        </p>
      </div>
    );
  }

  if (state.kind === "error") {
    return (
      <div>
        <Header />
        <p className="font-body text-[13px] text-status-error">
          Linear sorgu başarısız: {state.message}
        </p>
      </div>
    );
  }

  const { active, backlog, total } = state.data;

  return (
    <div>
      <Header total={total} />
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <QueueSection
          title="Aktif"
          subtitle="In Progress"
          accentClass="text-status-running"
          issues={active.slice(0, MAX_PER_SECTION)}
          totalCount={active.length}
          emptyMessage="Aktif iş yok."
        />
        <QueueSection
          title="Sıradaki"
          subtitle="Backlog / Todo"
          accentClass="text-status-waiting"
          issues={backlog.slice(0, MAX_PER_SECTION)}
          totalCount={backlog.length}
          emptyMessage="Sıradaki iş yok."
        />
      </div>
    </div>
  );
}

function Header({ total }: { total?: number }) {
  return (
    <h3 className="font-mono text-sm uppercase tracking-widest text-text-muted mb-3">
      AVK Linear Kuyruğu
      {typeof total === "number" && (
        <span className="ml-2 normal-case tracking-normal text-text-dim">
          · {total} aktif+sıradaki
        </span>
      )}
    </h3>
  );
}

function QueueSection({
  title,
  subtitle,
  accentClass,
  issues,
  totalCount,
  emptyMessage,
}: {
  title: string;
  subtitle: string;
  accentClass: string;
  issues: LinearIssue[];
  totalCount: number;
  emptyMessage: string;
}) {
  return (
    <section>
      <div className="flex items-baseline justify-between mb-2">
        <h4 className={`font-mono text-xs uppercase tracking-wider ${accentClass}`}>
          {title} ({totalCount})
        </h4>
        <span className="font-mono text-[10px] text-text-muted">{subtitle}</span>
      </div>
      {issues.length === 0 ? (
        <p className="font-body text-[12px] text-text-dim">{emptyMessage}</p>
      ) : (
        <ul className="space-y-1.5">
          {issues.map((issue) => (
            <IssueRow key={issue.id} issue={issue} />
          ))}
        </ul>
      )}
    </section>
  );
}

function IssueRow({ issue }: { issue: LinearIssue }) {
  const priorityClass = PRIORITY_CLASS[issue.priority] ?? PRIORITY_CLASS[0];
  const priorityLabel = issue.priority_label || PRIORITY_LABEL[issue.priority] || "Yok";
  return (
    <li className="rounded border border-surface-700 bg-surface-800 px-2.5 py-2">
      <div className="flex items-start gap-2">
        <span
          className={`font-mono text-[10px] uppercase tracking-wider border px-1.5 py-0.5 rounded shrink-0 ${priorityClass}`}
          title={`Priority ${issue.priority} — ${priorityLabel}`}
        >
          {priorityLabel}
        </span>
        <a
          href={issue.url || "#"}
          target="_blank"
          rel="noopener noreferrer"
          className="flex-1 min-w-0 font-body text-[13px] text-text-primary hover:text-brand-500 transition-colors"
          title={issue.title}
        >
          <span className="font-mono text-text-muted mr-1">{issue.identifier}</span>
          {issue.title}
        </a>
      </div>
      <div className="flex items-center justify-between mt-1 font-mono text-[10px] text-text-muted">
        <span>
          {issue.assignee ?? "atanmamış"}
          {issue.team_key && (
            <span className="ml-2 opacity-70">{issue.team_key}</span>
          )}
        </span>
        <span title={issue.updated_at}>{formatRelativeTime(issue.updated_at)}</span>
      </div>
    </li>
  );
}
