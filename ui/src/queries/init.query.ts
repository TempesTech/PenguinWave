import { request } from '@/lib/daemon';
import type { Snapshot } from '@proto/Snapshot';

/// `init_app` is gone: the daemon initialises itself. A cold client reads the
/// snapshot instead.
export async function snapshotQuery(): Promise<Snapshot> {
  return request({ method: 'session.snapshot' }, 'snapshot');
}
