/**
 * Linear widget — In Progress / Backlog / Done 7g + 5 recent.
 * Contract §1 (snake_case JSON).
 */
import { useLinearSummary } from "../../hooks/integrations";
import { CountBox, WidgetShell, formatTime } from "./WidgetShell";

export function LinearWidget() {
  const { result, isLoading } = useLinearSummary();
  return (
    <WidgetShell
      title="Linear"
      result={result}
      isLoading={isLoading}
      timestamp={result?.ok ? formatTime(result.data.fetched_at) : undefined}
    >
      {(data) => (
        <>
          <div className="flex flex-wrap gap-1.5 mb-2.5">
            <CountBox
              label="In Progress"
              value={data.in_progress.has_more ? `${data.in_progress.count}+` : data.in_progress.count}
              accent="waiting"
              hint={data.in_progress.has_more ? "250+ saturate" : undefined}
            />
            <CountBox
              label="Backlog"
              value={data.backlog.has_more ? `${data.backlog.count}+` : data.backlog.count}
              accent="info"
              hint={data.backlog.has_more ? "250+ saturate" : undefined}
            />
            <CountBox
              label="Done 7g"
              value={data.done7d.has_more ? `${data.done7d.count}+` : data.done7d.count}
              accent="running"
              hint={data.done7d.has_more ? "250+ saturate" : undefined}
            />
          </div>
          {data.recent.length === 0 ? (
            <p className="text-text-muted text-xs">Son güncel issue yok.</p>
          ) : (
            <ul className="flex flex-col gap-1">
              {data.recent.map((issue) => (
                <li key={issue.identifier} className="flex gap-2 items-baseline text-xs">
                  <a
                    href={issue.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-brand-500 hover:underline shrink-0 min-w-[76px]"
                  >
                    {issue.identifier}
                  </a>
                  <span className="text-text-primary flex-1 truncate">{issue.title}</span>
                  <span className="text-text-muted text-[10px] shrink-0">{issue.state}</span>
                </li>
              ))}
            </ul>
          )}
        </>
      )}
    </WidgetShell>
  );
}
