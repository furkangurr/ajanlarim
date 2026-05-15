/**
 * Sentry widget — Total 24h / Unresolved + 5 recent issue.
 * Contract §2 (snake_case JSON).
 */
import { useSentrySummary } from "../../hooks/integrations";
import { CountBox, WidgetShell, formatTime } from "./WidgetShell";

export function SentryWidget() {
  const { result, isLoading } = useSentrySummary();
  return (
    <WidgetShell
      title="Sentry"
      result={result}
      isLoading={isLoading}
      timestamp={result?.ok ? formatTime(result.data.fetched_at) : undefined}
    >
      {(data) => (
        <>
          <div className="flex flex-wrap gap-1.5 mb-2.5">
            <CountBox label="Total 24h" value={data.total_issues} accent="muted" />
            <CountBox label="Unresolved" value={data.unresolved} accent="error" />
          </div>
          {data.recent.length === 0 ? (
            <p className="text-text-muted text-xs">Son 24 saatte unresolved issue yok.</p>
          ) : (
            <ul className="flex flex-col gap-1">
              {data.recent.map((issue) => (
                <li key={issue.id} className="flex gap-2 items-baseline text-xs">
                  <a
                    href={issue.permalink}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-status-error hover:underline shrink-0 min-w-[96px]"
                  >
                    {issue.short_id}
                  </a>
                  <span className="text-text-primary flex-1 truncate">{issue.title}</span>
                  <span className="text-status-waiting text-[10px] shrink-0">{issue.count}x</span>
                </li>
              ))}
            </ul>
          )}
        </>
      )}
    </WidgetShell>
  );
}
