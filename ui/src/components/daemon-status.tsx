import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import { useDaemonConnection, useDaemonEvents } from '@/hooks/use-daemon';
import { enableService, serviceState, type UnitState } from '@/lib/daemon';

/// Disconnected banner.
///
/// Stale readings must never be presented as live: while the daemon is away
/// the UI says so rather than leaving the last values on screen unqualified.
///
/// When the unit is merely not enabled, the banner offers to start it. The
/// package cannot do that itself -- user units are per-user and package
/// scripts run as root -- so the offer has to come from here.
export function DaemonStatus({ children }: { children: React.ReactNode }) {
  useDaemonEvents();
  const connected = useDaemonConnection();
  const [unit, setUnit] = useState<UnitState>('unavailable');
  const [starting, setStarting] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);

  useEffect(() => {
    if (connected === false) {
      void serviceState().then(setUnit);
    }
  }, [connected]);

  const start = async () => {
    setStarting(true);
    setFailure(null);
    try {
      await enableService();
    } catch (error) {
      setFailure(error instanceof Error ? error.message : String(error));
    } finally {
      setStarting(false);
    }
  };

  const down = connected === false;

  return (
    <>
      {down && (
        <div
          role="status"
          className="sticky top-0 z-50 w-full bg-destructive px-4 py-2 text-center text-sm text-destructive-foreground"
        >
          <div className="flex flex-wrap items-center justify-center gap-x-3 gap-y-1">
            <span>
              Not connected to penguinwave-daemon. Values shown are the last known state and
              are not live.
            </span>
            {unit === 'disabled' ? (
              <Button
                size="sm"
                variant="secondary"
                className="h-6 text-[11px]"
                disabled={starting}
                onClick={() => void start()}
              >
                {starting ? 'Starting…' : 'Start the daemon'}
              </Button>
            ) : (
              <span className="opacity-80">Reconnecting…</span>
            )}
          </div>
          {failure && <div className="mt-1 text-[11px] opacity-90">{failure}</div>}
        </div>
      )}
      <div aria-hidden={down} className={down ? 'opacity-60' : ''}>
        {children}
      </div>
    </>
  );
}
