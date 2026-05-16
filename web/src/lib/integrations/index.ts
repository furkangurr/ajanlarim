/**
 * Widget API client common types + helper.
 *
 * Contract: docs/aoe-transplant/02-widget-api-contract.md (Code-1 dondurucu)
 * Endpoint base: /api/widgets/<service>/summary
 * Convention: snake_case JSON (Rust serde), 200/500/502, plain-text error body
 *
 * Code-2 sole — Adım 3 (FUR-3957 transplant).
 */

export type WidgetResult<T> =
  | { ok: true; data: T; cache: "HIT" | "MISS" | "unknown" }
  | { ok: false; status: number; error: string };

/**
 * GET widget endpoint with Code-1 contract pattern.
 * - 200: parse JSON, capture x-cache header
 * - 500/502: read plain-text body as error
 * - network fail: ok:false + status 0
 */
export async function fetchWidget<T>(endpoint: string): Promise<WidgetResult<T>> {
  try {
    const res = await fetch(endpoint, { cache: "no-store" });
    if (!res.ok) {
      const errText = await res.text().catch(() => res.statusText);
      return { ok: false, status: res.status, error: errText || `HTTP ${res.status}` };
    }
    const cache = (res.headers.get("x-cache") as "HIT" | "MISS" | null) ?? "unknown";
    const data = (await res.json()) as T;
    return { ok: true, data, cache };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { ok: false, status: 0, error: `network: ${message}` };
  }
}
