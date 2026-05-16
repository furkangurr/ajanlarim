/**
 * Vercel widget — Ready/Error/Building CountBox + recent deployment.
 * Contract §4 (snake_case JSON, created_at epoch ms).
 */
import { useVercelSummary } from "../../hooks/integrations";
import type { VercelDeployment } from "../../lib/integrations/vercel";
import { CountBox, WidgetShell, formatTime } from "./WidgetShell";

function stateColor(state: string): string {
  switch (state) {
    case "READY":
      return "text-status-running";
    case "ERROR":
      return "text-status-error";
    case "BUILDING":
    case "INITIALIZING":
      return "text-status-waiting";
    case "QUEUED":
      return "text-brand-500";
    case "CANCELED":
      return "text-text-muted";
    default:
      return "text-text-muted";
  }
}

function formatRelative(epochMs: number): string {
  const diff = Date.now() - epochMs;
  const m = Math.floor(diff / 60_000);
  if (m < 1) return "az önce";
  if (m < 60) return `${m}dk önce`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}sa önce`;
  return `${Math.floor(h / 24)}g önce`;
}

export function VercelWidget() {
  const { result, isLoading } = useVercelSummary();
  return (
    <WidgetShell
      title="Vercel"
      result={result}
      isLoading={isLoading}
      timestamp={result?.ok ? formatTime(result.data.fetched_at) : undefined}
    >
      {(data) => (
        <>
          <div className="flex flex-wrap gap-1.5 mb-2.5">
            <CountBox label="Ready" value={data.counts.ready} accent="running" />
            <CountBox label="Error" value={data.counts.error} accent="error" />
            <CountBox label="Building" value={data.counts.building} accent="waiting" />
          </div>
          {data.recent.length === 0 ? (
            <p className="text-text-muted text-xs">Son deployment yok.</p>
          ) : (
            <ul className="flex flex-col gap-1 border-t border-surface-700/40 pt-2">
              {data.recent.map((d: VercelDeployment) => (
                <li key={d.uid} className="flex gap-2 items-baseline text-[11px]">
                  <span className={`${stateColor(d.state)} shrink-0 min-w-[64px] font-semibold`}>
                    {d.state}
                  </span>
                  <a
                    href={d.url.startsWith("http") ? d.url : `https://${d.url}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-text-primary hover:underline flex-1 truncate"
                  >
                    {d.meta.branch ?? d.target ?? d.name}
                  </a>
                  <span className="text-text-muted text-[10px] shrink-0">
                    {formatRelative(d.created_at)}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </>
      )}
    </WidgetShell>
  );
}
