import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { AppNavigation } from '@/components/app-navigation';
import { PageHeader } from '@/components/page-header';
import { UserDevices } from '@/components/user-devices';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { useToast } from '@/hooks/use-toast';
import {
  GET_SYSTEM_DEPS_QUERY,
  GET_UDEV_STATUS_QUERY,
  installUdevRuleMutation,
} from '@/queries/system.query';
import {
  CheckCircle2,
  XCircle,
  RefreshCw,
  ShieldCheck,
  Package,
  Power,
} from 'lucide-react';

function StatusBadge({ ok }: { ok: boolean }) {
  return ok ? (
    <Badge className="gap-1 bg-green-500/15 text-green-400 border-green-500/30 hover:bg-green-500/20">
      <CheckCircle2 className="h-3 w-3" />
      OK
    </Badge>
  ) : (
    <Badge className="gap-1 bg-red-500/15 text-red-400 border-red-500/30 hover:bg-red-500/20">
      <XCircle className="h-3 w-3" />
      Missing
    </Badge>
  );
}

/// Shows what the daemon can actually see: PipeWire, pactl, pw-link and
/// libhidapi. There is no distro or install-command guess here any more --
/// that lived in the old Tauri command and had no equivalent on the wire.
function DepsSection() {
  const { data, isLoading, refetch, isRefetching } = useQuery(GET_SYSTEM_DEPS_QUERY);

  const rows: [string, boolean | undefined][] = [
    ['pipewire', data?.pipewire],
    ['pactl', data?.pactl],
    ['pw-link', data?.pw_link],
    ['libhidapi', data?.libhidapi],
  ];

  return (
    <section className="rounded-xl border border-border bg-card p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <Package className="h-5 w-5 text-primary" />
          <div>
            <h2 className="text-sm font-semibold">System Dependencies</h2>
            <p className="text-xs text-muted-foreground">
              What penguinwave-daemon can see on this machine
            </p>
          </div>
        </div>
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={() => refetch()}
          disabled={isLoading || isRefetching}
        >
          <RefreshCw className={`h-3.5 w-3.5 ${isRefetching ? 'animate-spin' : ''}`} />
        </Button>
      </div>

      {isLoading && (
        <div className="text-xs text-muted-foreground animate-pulse">Checking…</div>
      )}

      {data && (
        <div className="grid grid-cols-2 gap-3 text-xs">
          {rows.map(([name, ok]) => (
            <div
              key={name}
              className="flex items-center justify-between rounded-lg bg-muted/40 px-3 py-2"
            >
              <span className="font-mono text-foreground">{name}</span>
              <StatusBadge ok={!!ok} />
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function AutostartSection() {
  const { toast } = useToast();
  const qc = useQueryClient();

  const { data: enabled, isLoading } = useQuery<boolean>({
    queryKey: ['maintenance', 'autostart'],
    queryFn: () => isEnabled(),
    staleTime: 0,
  });

  const { mutate: toggle, isPending } = useMutation({
    mutationFn: async (next: boolean) => {
      if (next) await enable();
      else await disable();
      return next;
    },
    onSuccess: (next) => {
      toast({
        title: next ? 'Autostart enabled' : 'Autostart disabled',
        description: next
          ? 'PenguinWave will launch on login and stay in the tray.'
          : 'PenguinWave will no longer launch on login.',
      });
      qc.invalidateQueries({ queryKey: ['maintenance', 'autostart'] });
    },
    onError: (err: Error) => {
      toast({ title: 'Could not change autostart', description: err.message, variant: 'destructive' });
    },
  });

  return (
    <section className="rounded-xl border border-border bg-card p-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Power className="h-5 w-5 text-primary" />
          <div>
            <h2 className="text-sm font-semibold">Start at login</h2>
            <p className="text-xs text-muted-foreground">
              Launch PenguinWave on boot, minimized to the system tray.
            </p>
          </div>
        </div>
        <Switch
          checked={!!enabled}
          disabled={isLoading || isPending}
          onCheckedChange={(v) => toggle(v)}
        />
      </div>
    </section>
  );
}

/// Ownership here is split across the machine and the daemon: the daemon
/// reports whether the rule is installed, and the client -- this window --
/// is the one that runs the pkexec command the daemon hands back, since the
/// daemon itself never elevates.
function UdevSection() {
  const { toast } = useToast();
  const qc = useQueryClient();

  const { data, isLoading, refetch, isRefetching } = useQuery(GET_UDEV_STATUS_QUERY);

  const { mutate: installRule, isPending: isInstalling } = useMutation({
    mutationFn: installUdevRuleMutation,
    onSuccess: () => {
      toast({ title: 'udev rule installed' });
      qc.invalidateQueries({ queryKey: GET_UDEV_STATUS_QUERY.queryKey });
    },
    onError: (err: Error) => {
      toast({
        title: 'Installation failed',
        description: err.message,
        variant: 'destructive',
      });
    },
  });

  return (
    <section className="rounded-xl border border-border bg-card p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <ShieldCheck className="h-5 w-5 text-primary" />
          <div>
            <h2 className="text-sm font-semibold">udev Rules</h2>
            <p className="text-xs text-muted-foreground">
              Grants non-root HID device access to your headset
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {data && <StatusBadge ok={data.installed} />}
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => refetch()}
            disabled={isLoading || isRefetching}
          >
            <RefreshCw className={`h-3.5 w-3.5 ${isRefetching ? 'animate-spin' : ''}`} />
          </Button>
        </div>
      </div>

      {isLoading && (
        <div className="text-xs text-muted-foreground animate-pulse">Checking…</div>
      )}

      {data && !data.installed && (
        <div className="space-y-4">
          <div className="rounded-lg bg-yellow-500/10 border border-yellow-500/20 p-3">
            <p className="text-xs text-yellow-400 mb-1 font-medium">
              No rule installed — the headset needs HID permission to work
            </p>
            <p className="text-xs text-muted-foreground">
              After install, replug your headset for the rule to take effect.
            </p>
          </div>

          <Button
            size="sm"
            onClick={() => installRule()}
            disabled={isInstalling || !data.install_command}
            className="gap-2"
          >
            {isInstalling ? (
              <RefreshCw className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <ShieldCheck className="h-3.5 w-3.5" />
            )}
            {isInstalling ? 'Installing…' : 'Install udev rule'}
          </Button>

          <p className="text-xs text-muted-foreground">
            Uses <code className="font-mono">pkexec</code> — a graphical authentication dialog
            will appear.
          </p>
        </div>
      )}
    </section>
  );
}

export default function Maintenance() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <AppNavigation />
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 animate-in fade-in duration-300">
        <PageHeader
          title="Maintenance"
          subtitle="System dependencies and hardware access configuration."
        />
        <div className="max-w-2xl space-y-4">
          <DepsSection />
          <AutostartSection />
          <UserDevices />
          <UdevSection />
        </div>
      </div>
    </div>
  );
}
