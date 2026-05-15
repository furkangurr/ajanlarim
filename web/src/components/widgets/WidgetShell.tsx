/**
 * Common widget shell — title header + loading/error fallback + body slot.
 *
 * Tailwind 4 surface-* + brand-* + status-* palette (AoE canon, web/src/index.css).
 * Code-2 Adım 3 transplant — FUR-3957.
 */
import type { ReactNode } from "react";
import type { WidgetResult } from "../../lib/integrations";

interface WidgetShellProps<T> {
  title: string;
  result: WidgetResult<T> | null;
  isLoading: boolean;
  timestamp?: string;
  children: (data: T) => ReactNode;
  className?: string;
}

export function WidgetShell<T>({
  title,
  result,
  isLoading,
  timestamp,
  children,
  className = "",
}: WidgetShellProps<T>) {
  const cacheBadge =
    result?.ok && result.cache !== "unknown" ? (
      <span className="text-text-muted text-[10px] uppercase tracking-wider ml-2">
        {result.cache}
      </span>
    ) : null;

  return (
    <section
      aria-label={`${title} widget`}
      className={`bg-surface-900 border border-surface-700/40 rounded-lg p-3 text-sm text-text-secondary ${className}`}
    >
      <header className="flex justify-between items-baseline mb-2">
        <div className="flex items-baseline">
          <strong className="text-text-primary text-sm font-semibold">{title}</strong>
          {cacheBadge}
        </div>
        {timestamp ? (
          <span className="text-text-muted text-[11px]">{timestamp}</span>
        ) : null}
      </header>
      {isLoading && !result ? (
        <p className="text-text-muted text-xs">Yükleniyor…</p>
      ) : result?.ok === false ? (
        <ErrorPanel status={result.status} message={result.error} />
      ) : result?.ok === true ? (
        children(result.data)
      ) : null}
    </section>
  );
}

function ErrorPanel({ status, message }: { status: number; message: string }) {
  const label = status === 0 ? "network" : status === 500 ? "env eksik" : status === 502 ? "upstream" : `HTTP ${status}`;
  return (
    <div className="flex flex-col gap-1">
      <span className="text-status-error text-[11px] uppercase">{label}</span>
      <p className="text-text-muted text-xs break-words">{message}</p>
    </div>
  );
}

interface CountBoxProps {
  label: string;
  value: string | number;
  accent: "running" | "waiting" | "error" | "info" | "muted";
  hint?: string;
}

export function CountBox({ label, value, accent, hint }: CountBoxProps) {
  const accentMap = {
    running: "border-status-running",
    waiting: "border-status-waiting",
    error: "border-status-error",
    info: "border-brand-600",
    muted: "border-surface-700",
  } as const;
  return (
    <div
      className={`flex-1 min-w-[80px] bg-surface-850 rounded-md px-2.5 py-1.5 border-l-2 ${accentMap[accent]}`}
    >
      <div className="text-text-muted text-[10px] uppercase tracking-wider">{label}</div>
      <div className="text-text-primary text-xl font-semibold leading-none">{value}</div>
      {hint ? <div className="text-status-waiting text-[10px] mt-0.5">{hint}</div> : null}
    </div>
  );
}

export function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" });
}
