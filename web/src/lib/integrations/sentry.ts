/**
 * Sentry widget TS client.
 * Contract: docs/aoe-transplant/02-widget-api-contract.md §2
 */
import { fetchWidget, type WidgetResult } from "./index";

export interface SentryIssue {
  id: string;
  short_id: string;
  title: string;
  culprit: string;
  count: string;
  permalink: string;
  last_seen: string;
}

export interface SentrySummary {
  total_issues: number;
  unresolved: number;
  recent: SentryIssue[];
  fetched_at: string;
}

export function fetchSentrySummary(): Promise<WidgetResult<SentrySummary>> {
  return fetchWidget<SentrySummary>("/api/widgets/sentry/summary");
}
