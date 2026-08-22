// Daemon connection state and the event -> query-invalidation mapping.

import { useEffect, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import {
  isConnected,
  onConnectionChange,
  onDaemonEvent,
  type DaemonEventName,
} from '@/lib/daemon';

/// Query keys each daemon event invalidates.
const INVALIDATES: Record<DaemonEventName, string[][]> = {
  'stream.list_changed': [['getApplicationStreams'], ['getAudioCategories']],
  'sink.list_changed': [['getCustomVirtualSinks'], ['getAudioCategories'], ['getDefaultSink']],
  'graph.changed': [['getOutputDevices'], ['getNodePorts'], ['getAudioCategories']],
  'chatmix.changed': [['getChatMix']],
  'eq.state_changed': [['getEqState']],
  'eq.safe_mode': [['getEqState']],
  'device.attached': [['getSupportedDevices']],
  'device.detached': [['getSupportedDevices']],
  'device.selection_changed': [['getSelectedDevice'], ['getSupportedDevices']],
  'daemon.shutting_down': [],
};

/// Keep the cache in step with the daemon.
///
/// Every mutating method emits an event, including back to the client that
/// made it, so this is the only state-update path: no local echo, no manual
/// refetch after a mutation.
export function useDaemonEvents() {
  const queryClient = useQueryClient();

  useEffect(() => {
    const names = Object.keys(INVALIDATES) as DaemonEventName[];
    const unlisten = names.map((name) =>
      onDaemonEvent(name, () => {
        for (const queryKey of INVALIDATES[name]) {
          void queryClient.invalidateQueries({ queryKey });
        }
      }),
    );
    return () => {
      unlisten.forEach((p) => void p.then((off) => off()));
    };
  }, [queryClient]);
}

/// Whether the daemon socket is up.
///
/// On reconnect the whole cache is dropped rather than merged: merging leaves
/// ghost streams that no longer exist.
export function useDaemonConnection() {
  const queryClient = useQueryClient();
  const [connected, setConnected] = useState<boolean | null>(null);

  useEffect(() => {
    let active = true;
    void isConnected().then((value) => {
      if (active) setConnected(value);
    });

    const unlisten = onConnectionChange((value) => {
      setConnected(value);
      if (value) {
        void queryClient.resetQueries();
      }
    });

    return () => {
      active = false;
      void unlisten.then((off) => off());
    };
  }, [queryClient]);

  return connected;
}
