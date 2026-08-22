import { useDaemonConnection, useDaemonEvents } from '@/hooks/use-daemon';

/// Disconnected banner.
///
/// Stale readings must never be presented as live: while the daemon is away
/// the UI says so rather than leaving the last values on screen unqualified.
export function DaemonStatus({ children }: { children: React.ReactNode }) {
  useDaemonEvents();
  const connected = useDaemonConnection();

  return (
    <>
      {connected === false && (
        <div
          role='status'
          className='sticky top-0 z-50 w-full bg-destructive px-4 py-2 text-center text-sm text-destructive-foreground'
        >
          Not connected to penguinwave-daemon. Values shown are the last known
          state and are not live. Reconnecting…
        </div>
      )}
      <div aria-hidden={connected === false} className={connected === false ? 'opacity-60' : ''}>
        {children}
      </div>
    </>
  );
}
