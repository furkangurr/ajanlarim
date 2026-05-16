/**
 * Netdata widget TS client.
 * Contract: docs/aoe-transplant/02-widget-api-contract.md §5
 */
import { fetchWidget, type WidgetResult } from "./index";

export interface NetdataChart {
  id: string;
  url: string;
}

export interface NetdataSummary {
  reachable: boolean;
  version: string;
  host: string;
  charts: NetdataChart[];
  iframe_base: string;
  fetched_at: string;
}

export function fetchNetdataSummary(): Promise<WidgetResult<NetdataSummary>> {
  return fetchWidget<NetdataSummary>("/api/widgets/netdata/summary");
}
