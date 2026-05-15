/**
 * GitHub Actions widget — repo-bazlı ✓✗▶ sayım + cross-repo recent run.
 * Contract §3 (snake_case JSON).
 */
import { useGitHubActionsSummary } from "../../hooks/integrations";
import type { GitHubRun } from "../../lib/integrations/github";
import { WidgetShell, formatTime } from "./WidgetShell";

function statusColor(run: GitHubRun): string {
  if (run.status === "in_progress" || run.status === "queued" || run.status === "waiting") {
    return "text-status-waiting";
  }
  if (run.conclusion === "success") return "text-status-running";
  if (run.conclusion === "failure" || run.conclusion === "timed_out") return "text-status-error";
  return "text-text-muted";
}

function statusLabel(run: GitHubRun): string {
  if (run.status === "in_progress") return "RUNNING";
  if (run.status === "queued" || run.status === "waiting") return run.status.toUpperCase();
  return (run.conclusion ?? "—").toUpperCase();
}

export function GithubActionsWidget() {
  const { result, isLoading } = useGitHubActionsSummary();
  return (
    <WidgetShell
      title="GitHub Actions"
      result={result}
      isLoading={isLoading}
      timestamp={result?.ok ? formatTime(result.data.fetched_at) : undefined}
    >
      {(data) => (
        <>
          <ul className="flex flex-col gap-0.5 mb-2">
            {data.repos.map((r) => (
              <li key={r.repo} className="flex gap-1.5 items-baseline text-xs">
                <span className="flex-1 truncate">{r.repo}</span>
                <span className="text-status-running" title="success">
                  ✓{r.success}
                </span>
                <span className="text-status-error" title="failure">
                  ✗{r.failure}
                </span>
                <span className="text-status-waiting" title="in progress">
                  ▶{r.in_progress}
                </span>
                {r.other > 0 ? (
                  <span className="text-text-muted" title="other">
                    ·{r.other}
                  </span>
                ) : null}
              </li>
            ))}
          </ul>
          {data.recent.length > 0 ? (
            <ul className="flex flex-col gap-1 border-t border-surface-700/40 pt-2">
              {data.recent.map((run) => (
                <li key={run.id} className="flex gap-2 items-baseline text-[11px]">
                  <span className={`${statusColor(run)} shrink-0 min-w-[64px] font-semibold`}>
                    {statusLabel(run)}
                  </span>
                  <a
                    href={run.html_url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-text-primary hover:underline flex-1 truncate"
                  >
                    {run.repo.split("/")[1]} · {run.name}
                  </a>
                  <span className="text-text-muted text-[10px] shrink-0">{run.head_branch}</span>
                </li>
              ))}
            </ul>
          ) : null}
        </>
      )}
    </WidgetShell>
  );
}
