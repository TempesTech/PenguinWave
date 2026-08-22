import { invoke } from '@tauri-apps/api/core';
import { Query } from './query';
import { request } from '@/lib/daemon';
import type { SystemDeps } from '@proto/SystemDeps';
import type { UdevStatus } from '@proto/UdevStatus';

const QUERY_KEYS = {
  GET_SYSTEM_DEPS: 'getSystemDeps',
  GET_UDEV_STATUS: 'getUdevStatus',
} as const;

export const GET_SYSTEM_DEPS_QUERY: Query<SystemDeps> = {
  queryKey: [QUERY_KEYS.GET_SYSTEM_DEPS],
  queryFn: async () => request({ method: 'system.check_deps' }, 'system_deps'),
};

export const GET_UDEV_STATUS_QUERY: Query<UdevStatus> = {
  queryKey: [QUERY_KEYS.GET_UDEV_STATUS],
  queryFn: async () => request({ method: 'system.check_udev' }, 'udev_status'),
};

/// Ask the daemon for the install command, then run it here.
///
/// The daemon never elevates itself -- it only ever returns argv starting
/// with `pkexec` -- so the client that runs it is the one place in the UI
/// that can prompt for a password, and only from this explicit action.
export async function installUdevRuleMutation(): Promise<string> {
  const status = await request({ method: 'system.install_udev' }, 'udev_status');
  if (!status.install_command) {
    throw new Error('the daemon reports the rule is already installed');
  }
  return invoke<string>('run_privileged_command', { argv: status.install_command });
}
