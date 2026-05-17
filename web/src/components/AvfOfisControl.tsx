/**
 * avfurkangur.com Ofisi başlat widget — 2 ajan (Koord + Code) tmux session.
 *
 * Tek buton: "▶ avfurkangur Ofisi Başlat". Backend `/root/bin/
 * avfurkangur-start` çağırır. Idempotent — mevcut session varsa atlar.
 * AvkOfisControl pattern ile aynı UX.
 */

import { useState } from "react";
import { postAvfOfisBaslat } from "../lib/api";
import type { AvfOfisBaslatResponse } from "../lib/types";

export function AvfOfisControl() {
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<AvfOfisBaslatResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function handleStart() {
    if (running) return;
    if (!confirm("avfurkangur.com 2-ajan ofisi (Koord + Code) başlatılsın mı?")) return;
    setRunning(true);
    setError(null);
    setResult(null);
    try {
      const res = await postAvfOfisBaslat();
      if (!res) {
        setError("Endpoint ulaşılamadı (/api/avk/avf-ofis-baslat)");
      } else {
        setResult(res);
        if (!res.ok) {
          setError(res.error ?? "Script başarısız döndü");
        }
      }
    } finally {
      setRunning(false);
    }
  }

  return (
    <div>
      <h3 className="font-mono text-sm uppercase tracking-widest text-text-muted mb-1">
        avfurkangur.com Ofisi
      </h3>
      <p className="font-body text-[12px] text-text-dim mb-3">
        SOL Koord + SAĞ Code Agent (2 pane). tmux session `avfurkangur`.
        Mevcut session varsa atlanır.
      </p>

      <button
        onClick={handleStart}
        disabled={running}
        className="rounded-lg bg-status-running/20 border border-status-running/40 text-status-running font-mono text-[13px] px-4 py-2 hover:bg-status-running/30 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        {running ? "Kuruluyor… (10-30s)" : "▶ Ofisi Başlat"}
      </button>

      {running && (
        <p className="font-mono text-[11px] text-text-muted mt-2">
          tmux pane'ler kuruluyor + claude CLI başlatılıyor…
        </p>
      )}

      {error && !running && (
        <p className="font-body text-[12px] text-status-error mt-3">
          ❌ {error}
        </p>
      )}

      {result && !running && (
        <div className="mt-3 space-y-2">
          <div className="flex items-center gap-3 text-[12px] font-mono">
            <span className={result.ok ? "text-status-running" : "text-status-error"}>
              {result.ok ? "✓ başarılı" : "✗ başarısız"}
            </span>
            <span className="text-text-muted">
              {(result.elapsed_ms / 1000).toFixed(1)}s
            </span>
          </div>
          {result.stdout_tail && (
            <pre className="font-mono text-[11px] text-text-secondary bg-surface-800 border border-surface-700 rounded p-3 overflow-x-auto whitespace-pre-wrap max-h-48">
{result.stdout_tail}
            </pre>
          )}
          {result.stderr_tail && (
            <pre className="font-mono text-[11px] text-status-error bg-surface-800 border border-status-error/40 rounded p-3 overflow-x-auto whitespace-pre-wrap max-h-32">
{result.stderr_tail}
            </pre>
          )}
        </div>
      )}
    </div>
  );
}
