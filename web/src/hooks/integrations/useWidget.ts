/**
 * Common widget hook factory — periodic polling fetch with WidgetResult state.
 *
 * Code-1 contract: 60s in-process cache backend tarafı; UI poll 30s default
 * (cache MISS ihtimali ≤50%). PreviousData korunur fetch sırasında — flicker
 * önleme.
 */
import { useEffect, useRef, useState } from "react";
import type { WidgetResult } from "../../lib/integrations";

export interface UseWidgetState<T> {
  result: WidgetResult<T> | null;
  isLoading: boolean;
}

export function useWidget<T>(
  fetcher: () => Promise<WidgetResult<T>>,
  pollMs: number = 30_000,
): UseWidgetState<T> {
  const [result, setResult] = useState<WidgetResult<T> | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    let timer: ReturnType<typeof setTimeout> | null = null;

    const tick = async () => {
      const r = await fetcher();
      if (!mountedRef.current) return;
      setResult(r);
      setIsLoading(false);
      timer = setTimeout(tick, pollMs);
    };

    tick();

    return () => {
      mountedRef.current = false;
      if (timer) clearTimeout(timer);
    };
    // fetcher kimliği kararlı varsayılır (modüle bağlı export); pollMs prop değişiminde re-init
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pollMs]);

  return { result, isLoading };
}
