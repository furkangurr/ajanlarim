/**
 * GitHub Actions widget TS client.
 * Contract: docs/aoe-transplant/02-widget-api-contract.md §3
 */
import { fetchWidget, type WidgetResult } from "./index";

export interface GitHubRepoStats {
  repo: string;
  success: number;
  failure: number;
  in_progress: number;
  other: number;
}

export interface GitHubRun {
  id: number;
  repo: string;
  name: string;
  status: string;
  conclusion: string | null;
  html_url: string;
  head_branch: string;
  event: string;
  run_started_at: string;
  updated_at: string;
}

export interface GitHubActionsSummary {
  repos: GitHubRepoStats[];
  recent: GitHubRun[];
  fetched_at: string;
}

export function fetchGitHubActionsSummary(): Promise<WidgetResult<GitHubActionsSummary>> {
  return fetchWidget<GitHubActionsSummary>("/api/widgets/github-actions/summary");
}
