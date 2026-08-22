import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { AppNavigation } from '@/components/app-navigation';
import { PageHeader } from '@/components/page-header';
import { UserDevices } from '@/components/user-devices';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { useToast } from '@/hooks/use-toast';
import {
  CheckCircle2,
  XCircle,
  AlertTriangle,
  RefreshCw,
  ShieldCheck,
  Package,
  Power,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';

interface SystemDepsStatus {
  hidapi_found: boolean;
  hidapi_version: string | null;
  distro: string;
  distro_id: string;
  install_command: string | null;
}

interface DeviceRule {
  vendor_id: string;
  product_id: string;
  vendor_name: string;
  product_name: string;
  rule_hidraw: string;
}

interface UdevRulesStatus {
  status: 'Missing' | 'Outdated' | 'Ok';
  current_content: string | null;
  expected_content: string;
  rules_path: string;
  device_rules: DeviceRule[];
}

function StatusBadge({ status }: { status: 'ok' | 'warn' | 'error' }) {
  if (status === 'ok')
    return (
      <Badge className="gap-1 bg-green-500/15 text-green-400 border-green-500/30 hover:bg-green-500/20">
        <CheckCircle2 className="h-3 w-3" />
        OK
      </Badge>
    );
  if (status === 'warn')
    return (
      <Badge className="gap-1 bg-yellow-500/15 text-yellow-400 border-yellow-500/30 hover:bg-yellow-500/20">
        <AlertTriangle className="h-3 w-3" />
        Outdated
      </Badge>
    );
  return (
    <Badge className="gap-1 bg-red-500/15 text-red-400 border-red-500/30 hover:bg-red-500/20">
      <XCircle className="h-3 w-3" />
      Missing
    </Badge>
  );
}

function DepsSection() {
  const { data, isLoading, refetch, isRefetching } = useQuery<SystemDepsStatus>({
    queryKey: ['maintenance', 'system-deps'],
    queryFn: () => invoke<SystemDepsStatus>('check_system_deps'),
    staleTime: 0,
  });

  const status: 'ok' | 'error' = data?.hidapi_found ? 'ok' : 'error';

  return (
    <section className="rounded-xl border border-border bg-card p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <Package className="h-5 w-5 text-primary" />
          <div>
            <h2 className="text-sm font-semibold">System Library</h2>
            <p className="text-xs text-muted-foreground">libhidapi — required for headset HID communication</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {data && <StatusBadge status={status} />}
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

      {data && (
        <div className="space-y-3">
          <div className="grid grid-cols-2 gap-3 text-xs">
            <div className="rounded-lg bg-muted/40 px-3 py-2">
              <span className="text-muted-foreground">Library</span>
              <p className="font-mono mt-0.5 text-foreground">
                {data.hidapi_version ?? 'libhidapi (not found)'}
              </p>
            </div>
            <div className="rounded-lg bg-muted/40 px-3 py-2">
              <span className="text-muted-foreground">Distribution</span>
              <p className="font-mono mt-0.5 text-foreground truncate">{data.distro}</p>
            </div>
          </div>

          {!data.hidapi_found && (
            <div className="rounded-lg bg-destructive/10 border border-destructive/20 p-3 space-y-2">
              <p className="text-xs font-medium text-destructive">Library not found — install it first</p>
              {data.install_command ? (
                <code className="block text-xs font-mono bg-background/60 rounded px-2 py-1.5 text-foreground select-all">
                  {data.install_command}
                </code>
              ) : (
                <p className="text-xs text-muted-foreground">
                  Install <code className="font-mono">libhidapi</code> via your distro's package manager.
                </p>
              )}
            </div>
          )}
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

function RuleDiff({
  current,
  expected,
}: {
  current: string | null;
  expected: string;
}) {
  const [showExpected, setShowExpected] = useState(false);

  const expectedLines = expected.split('\n');
  const currentLines = current?.split('\n') ?? [];

  return (
    <div className="space-y-2">
      {current && (
        <div>
          <button
            onClick={() => setShowExpected((v) => !v)}
            className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
          >
            {showExpected ? <ChevronUp className="h-3 w-3" /> : <ChevronDown className="h-3 w-3" />}
            {showExpected ? 'Hide' : 'Show'} current / expected diff
          </button>

          {showExpected && (
            <div className="mt-2 grid grid-cols-2 gap-2">
              <div>
                <p className="text-xs text-muted-foreground mb-1">Current</p>
                <pre className="text-xs font-mono bg-muted/40 rounded-lg p-2 overflow-x-auto leading-relaxed">
                  {currentLines.map((line, i) => {
                    const inExpected = expectedLines.some(
                      (el) => el.trim() === line.trim() || line.trim() === '' || line.startsWith('#'),
                    );
                    return (
                      <span
                        key={i}
                        className={`block ${!inExpected ? 'text-red-400' : 'text-foreground'}`}
                      >
                        {line || ' '}
                      </span>
                    );
                  })}
                </pre>
              </div>
              <div>
                <p className="text-xs text-muted-foreground mb-1">Expected</p>
                <pre className="text-xs font-mono bg-muted/40 rounded-lg p-2 overflow-x-auto leading-relaxed">
                  {expectedLines.map((line, i) => {
                    const inCurrent = currentLines.some(
                      (cl) => cl.trim() === line.trim() || line.trim() === '' || line.startsWith('#'),
                    );
                    return (
                      <span
                        key={i}
                        className={`block ${!inCurrent ? 'text-green-400' : 'text-foreground'}`}
                      >
                        {line || ' '}
                      </span>
                    );
                  })}
                </pre>
              </div>
            </div>
          )}
        </div>
      )}

      {!current && (
        <div>
          <p className="text-xs text-muted-foreground mb-1">Rule to be installed</p>
          <pre className="text-xs font-mono bg-muted/40 rounded-lg p-3 overflow-x-auto leading-relaxed text-foreground">
            {expected}
          </pre>
        </div>
      )}
    </div>
  );
}

function UdevSection() {
  const { toast } = useToast();
  const qc = useQueryClient();

  const { data, isLoading, refetch, isRefetching } = useQuery<UdevRulesStatus>({
    queryKey: ['maintenance', 'udev-rules'],
    queryFn: () => invoke<UdevRulesStatus>('check_udev_rules'),
    staleTime: 0,
  });

  const { mutate: installRules, isPending: isInstalling } = useMutation({
    mutationFn: () => invoke<string>('install_udev_rules'),
    onSuccess: (msg) => {
      toast({ title: 'udev rules installed', description: msg });
      qc.invalidateQueries({ queryKey: ['maintenance', 'udev-rules'] });
    },
    onError: (err: Error) => {
      toast({
        title: 'Installation failed',
        description: err.message,
        variant: 'destructive',
      });
    },
  });

  const udevStatus: 'ok' | 'warn' | 'error' =
    data?.status === 'Ok' ? 'ok' : data?.status === 'Outdated' ? 'warn' : 'error';

  const needsAction = data?.status === 'Missing' || data?.status === 'Outdated';
  const btnLabel =
    data?.status === 'Missing'
      ? 'Install udev rule'
      : data?.status === 'Outdated'
        ? 'Update udev rule'
        : 'Reinstall udev rule';

  return (
    <section className="rounded-xl border border-border bg-card p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <ShieldCheck className="h-5 w-5 text-primary" />
          <div>
            <h2 className="text-sm font-semibold">udev Rules</h2>
            <p className="text-xs text-muted-foreground">
              Grants non-root HID device access ({data?.rules_path ?? RULES_PATH})
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {data && <StatusBadge status={udevStatus} />}
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

      {data && (
        <div className="space-y-4">
          {data.device_rules.length > 0 && (
            <div>
              <p className="text-xs text-muted-foreground mb-2">Covered devices</p>
              <div className="flex flex-wrap gap-2">
                {data.device_rules.map((dr) => (
                  <div
                    key={`${dr.vendor_id}:${dr.product_id}`}
                    className="flex items-center gap-2 rounded-lg bg-muted/40 px-3 py-1.5"
                  >
                    <span className="text-xs font-medium text-foreground">
                      {dr.vendor_name} {dr.product_name}
                    </span>
                    <code className="text-xs font-mono text-muted-foreground">
                      {dr.vendor_id}:{dr.product_id}
                    </code>
                  </div>
                ))}
              </div>
            </div>
          )}

          <RuleDiff current={data.current_content} expected={data.expected_content} />

          {data.status !== 'Ok' && (
            <div className="rounded-lg bg-yellow-500/10 border border-yellow-500/20 p-3">
              <p className="text-xs text-yellow-400 mb-1 font-medium">
                {data.status === 'Missing'
                  ? 'No rule file found — headset requires HID permission to function'
                  : 'Rule file is outdated — update to add missing devices'}
              </p>
              <p className="text-xs text-muted-foreground">
                After install, replug your headset for the rule to take effect.
              </p>
            </div>
          )}

          <Button
            size="sm"
            variant={needsAction ? 'default' : 'outline'}
            className="gap-2"
            onClick={() => installRules()}
            disabled={isInstalling}
          >
            {isInstalling ? (
              <RefreshCw className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <ShieldCheck className="h-3.5 w-3.5" />
            )}
            {isInstalling ? 'Installing…' : btnLabel}
          </Button>

          <p className="text-xs text-muted-foreground">
            Uses <code className="font-mono">pkexec</code> — a graphical authentication dialog will appear.
          </p>
        </div>
      )}
    </section>
  );
}

const RULES_PATH = '/etc/udev/rules.d/99-penguinwave.rules';

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
