/**
 * Linear widget TS client.
 * Contract: docs/aoe-transplant/02-widget-api-contract.md §1
 */
import { fetchWidget, type WidgetResult } from "./index";

export interface LinearCount {
  count: number;
  has_more: boolean;
}

export interface LinearIssue {
  identifier: string;
  title: string;
  state: string;
  url: string;
  updated_at: string;
}

export interface LinearSummary {
  in_progress: LinearCount;
  backlog: LinearCount;
  done7d: LinearCount;
  recent: LinearIssue[];
  fetched_at: string;
}

export function fetchLinearSummary(): Promise<WidgetResult<LinearSummary>> {
  return fetchWidget<LinearSummary>("/api/widgets/linear/summary");
}
