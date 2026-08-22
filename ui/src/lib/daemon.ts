// The only path from the UI to the audio stack.
//
// Every call goes to penguinwave-daemon over its socket; the Tauri layer is a
// passthrough. Types come from penguinwave-proto, so a protocol change is a
// compile error here rather than a runtime `undefined`.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Request } from '@proto/Request';
import type { Response } from '@proto/Response';
import type { PwError } from '@proto/PwError';
import type { Event as DaemonEvent } from '@proto/Event';

export type ResponseKind = Response['kind'];
export type PayloadOf<K extends ResponseKind> = Extract<Response, { kind: K }> extends {
  data: infer D;
}
  ? D
  : void;

// Tauri rejects event names containing a dot and every protocol name has one,
// so all daemon events arrive on one channel and are dispatched here.
export const DAEMON_EVENT = 'daemon:event';
export const CONNECTION_EVENT = 'daemon:connection';

export class DaemonError extends Error {
  constructor(readonly detail: PwError) {
    super(detail.msg);
    this.name = 'DaemonError';
  }

  get kind() {
    return this.detail.kind;
  }

  get retryable() {
    return this.detail.retryable;
  }
}

function isPwError(value: unknown): value is PwError {
  return typeof value === 'object' && value !== null && 'kind' in value && 'msg' in value;
}

/// Send a request and assert the response kind.
///
/// The daemon tags every payload, so a mismatch means a client bug rather than
/// a silently mis-parsed list.
export async function request<K extends ResponseKind>(
  req: Request,
  expect: K,
): Promise<PayloadOf<K>> {
  let response: Response;
  try {
    response = await invoke<Response>('daemon_request', { request: req });
  } catch (raw) {
    throw isPwError(raw) ? new DaemonError(raw) : new Error(String(raw));
  }

  if (response.kind !== expect) {
    throw new Error(`expected a ${expect} response, got ${response.kind}`);
  }
  return (response as { data?: unknown }).data as PayloadOf<K>;
}

/// Fire-and-check request for methods whose result the UI ignores.
export async function send(req: Request): Promise<void> {
  try {
    await invoke<Response>('daemon_request', { request: req });
  } catch (raw) {
    throw isPwError(raw) ? new DaemonError(raw) : new Error(String(raw));
  }
}

export async function isConnected(): Promise<boolean> {
  return invoke<boolean>('daemon_connected');
}

/// Whether the daemon's user unit is enabled.
///
/// `unavailable` means there is nothing to offer: no systemd session, or the
/// daemon package is not installed.
export type UnitState = 'enabled' | 'disabled' | 'unavailable';

export async function serviceState(): Promise<UnitState> {
  return invoke<UnitState>('daemon_service_state');
}

export async function enableService(): Promise<void> {
  return invoke<void>('enable_daemon_service');
}

export type DaemonEventName = DaemonEvent['event'];

/// Subscribe to one daemon event by its protocol name.
export function onDaemonEvent<N extends DaemonEventName>(
  name: N,
  handler: (event: Extract<DaemonEvent, { event: N }>) => void,
) {
  return listen<DaemonEvent>(DAEMON_EVENT, (e) => {
    if (e.payload.event === name) {
      handler(e.payload as Extract<DaemonEvent, { event: N }>);
    }
  });
}

/// Subscribe to every daemon event.
export function onAnyDaemonEvent(handler: (event: DaemonEvent) => void) {
  return listen<DaemonEvent>(DAEMON_EVENT, (e) => handler(e.payload));
}

export function onConnectionChange(handler: (connected: boolean) => void) {
  return listen<boolean>(CONNECTION_EVENT, (e) => handler(e.payload));
}
