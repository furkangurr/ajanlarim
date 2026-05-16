/**
 * Widget hooks barrel — Code-2 Adım 3 transplant.
 * Contract: docs/aoe-transplant/02-widget-api-contract.md
 */
import { useWidget } from "./useWidget";
import { fetchLinearSummary, type LinearSummary } from "../../lib/integrations/linear";
import { fetchSentrySummary, type SentrySummary } from "../../lib/integrations/sentry";
import {
  fetchGitHubActionsSummary,
  type GitHubActionsSummary,
} from "../../lib/integrations/github";
import { fetchVercelSummary, type VercelSummary } from "../../lib/integrations/vercel";
import { fetchNetdataSummary, type NetdataSummary } from "../../lib/integrations/netdata";

export function useLinearSummary() {
  return useWidget<LinearSummary>(fetchLinearSummary);
}

export function useSentrySummary() {
  return useWidget<SentrySummary>(fetchSentrySummary);
}

export function useGitHubActionsSummary() {
  return useWidget<GitHubActionsSummary>(fetchGitHubActionsSummary);
}

export function useVercelSummary() {
  return useWidget<VercelSummary>(fetchVercelSummary);
}

export function useNetdataSummary() {
  return useWidget<NetdataSummary>(fetchNetdataSummary);
}

export { useWidget };
