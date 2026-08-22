import { useCallback, useEffect, useRef } from 'react';

export function useDebouncedMutation<TVars>(
  mutate: (vars: TVars) => void,
  delayMs: number,
) {
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mutateRef = useRef(mutate);

  useEffect(() => {
    mutateRef.current = mutate;
  }, [mutate]);

  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, []);

  return useCallback(
    (vars: TVars) => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
      timeoutRef.current = setTimeout(() => {
        mutateRef.current(vars);
        timeoutRef.current = null;
      }, delayMs);
    },
    [delayMs],
  );
}
