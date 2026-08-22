import { useCallback, useState } from 'react';

const STORAGE_KEY = 'penguin-wave-stream-labels';

function read(): Record<string, string> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    return typeof parsed === 'object' && parsed !== null ? parsed : {};
  } catch {
    return {};
  }
}

function write(map: Record<string, string>): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(map));
}

export function useStreamLabels() {
  const [labels, setLabels] = useState<Record<string, string>>(() => read());

  const setLabel = useCallback((key: string, value: string) => {
    setLabels((prev) => {
      const next = { ...prev };
      const trimmed = value.trim();
      if (trimmed) next[key] = trimmed;
      else delete next[key];
      write(next);
      return next;
    });
  }, []);

  const clearLabel = useCallback((key: string) => {
    setLabels((prev) => {
      if (!(key in prev)) return prev;
      const next = { ...prev };
      delete next[key];
      write(next);
      return next;
    });
  }, []);

  return { labels, setLabel, clearLabel };
}
