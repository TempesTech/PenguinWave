// Kept for components that listen to a single daemon event by name.
//
// Prefer `useDaemonEvents` for cache upkeep; this is for components that need
// the payload itself.

import { useEffect } from 'react';
import { onDaemonEvent, type DaemonEventName } from '@/lib/daemon';
import type { Event as DaemonEvent } from '@proto/Event';

export function useDaemonEvent<N extends DaemonEventName>(
  name: N,
  handler: (event: Extract<DaemonEvent, { event: N }>) => void,
  deps: React.DependencyList = [],
) {
  useEffect(() => {
    const unlisten = onDaemonEvent(name, handler);
    return () => {
      void unlisten.then((off) => off());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
}
