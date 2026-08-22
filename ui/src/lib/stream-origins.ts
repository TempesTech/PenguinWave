/**
 * In-memory map of stream id → the sink it was on BEFORE first being routed
 * into a custom virtual sink. Used by Unassign to restore the original sink
 * instead of always falling back to the system default.
 *
 * Not persisted: PipeWire stream ids (object.serial) change every session,
 * so cross-session storage would be meaningless.
 */
const origins = new Map<number, number>();

/** Record only the FIRST origin. Subsequent moves between custom sinks do not overwrite. */
export function rememberOrigin(streamId: number, originSinkId: number): void {
  if (!origins.has(streamId)) origins.set(streamId, originSinkId);
}

/** Read + delete in one call. Returns undefined if no origin was tracked. */
export function consumeOrigin(streamId: number): number | undefined {
  const value = origins.get(streamId);
  origins.delete(streamId);
  return value;
}

/** Forget all tracked origins (e.g. on app reload). */
export function clearOrigins(): void {
  origins.clear();
}
