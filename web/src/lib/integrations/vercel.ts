/**
 * Vercel widget TS client.
 * Contract: docs/aoe-transplant/02-widget-api-contract.md §4
 */
import { fetchWidget, type WidgetResult } from "./index";

export interface VercelStateCounts {
  ready: number;
  error: number;
  building: number;
  queued: number;
  canceled: number;
  other: number;
}

export interface VercelDeployment {
  uid: string;
  name: string;
  url: string;
  state: string;
  target: string | null;
  created_at: number;
  source: string | null;
  meta: {
    branch?: string;
    commit_sha?: string;
  };
}

export interface VercelSummary {
  counts: VercelStateCounts;
  recent: VercelDeployment[];
  fetched_at: string;
}

export function fetchVercelSummary(): Promise<WidgetResult<VercelSummary>> {
  return fetchWidget<VercelSummary>("/api/widgets/vercel/summary");
}
