/**
 * Netdata widget — backend reachability + iframe embed (full-width).
 * Contract §5 (snake_case JSON, reachable + iframe_base + chart URL listesi).
 *
 * Backend down ise iframe placeholder, up ise iframe embed.
 */
import { useNetdataSummary } from "../../hooks/integrations";
import { WidgetShell, formatTime } from "./WidgetShell";

export function NetdataWidget() {
  const { result, isLoading } = useNetdataSummary();
  return (
    <WidgetShell
      title="Netdata"
      result={result}
      isLoading={isLoading}
      timestamp={result?.ok ? formatTime(result.data.fetched_at) : undefined}
      className="col-span-full"
    >
      {(data) => (
        <>
          <header className="flex justify-between items-baseline mb-2 text-[11px]">
            <span className={data.reachable ? "text-status-running" : "text-status-error"}>
              {data.reachable ? "reachable" : "down"} · {data.version} · {data.host}
            </span>
            <a
              href={data.iframe_base}
              target="_blank"
              rel="noopener noreferrer"
              className="text-text-muted hover:text-text-primary"
            >
              tam panel ↗
            </a>
          </header>
          {data.reachable ? (
            <iframe
              src={data.iframe_base}
              title="Netdata embedded dashboard"
              loading="lazy"
              sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
              className="w-full min-h-[360px] bg-surface-950 border border-surface-700/40 rounded-md"
            />
          ) : (
            <p className="text-text-muted text-xs">
              Netdata erişilemez ({data.host}). Backend `<code>{data.iframe_base}</code>` boş.
            </p>
          )}
        </>
      )}
    </WidgetShell>
  );
}
