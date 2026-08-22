import { Query } from '@/queries/query.ts';
import { ApplicationStream } from '@/types/audio.ts';
import { request, send } from '@/lib/daemon';
import { sinkNameById, streamRef, streamsByApp } from '@/lib/adapters';

const QUERY_KEYS = {
  GET_APPLICATION_STREAMS: 'getApplicationStreams',
};

export const GET_APPLICATION_STREAMS_QUERY: Query<Record<string, ApplicationStream[]>> = {
  queryKey: [QUERY_KEYS.GET_APPLICATION_STREAMS],
  queryFn: async () => {
    const [streams, sinks] = await Promise.all([
      request({ method: 'stream.list' }, 'streams'),
      request({ method: 'sink.list_custom' }, 'sinks'),
    ]);
    return streamsByApp(streams, sinks);
  },
};

export async function moveApplicationStreamToSinkMutation(
  streamIndex: number,
  sinkId: number,
): Promise<void> {
  await send({
    method: 'stream.move',
    params: { stream: streamRef(streamIndex), sink: await sinkNameById(sinkId) },
  });
}

export async function setStreamVolumeMutation(
  streamIndex: number,
  volume: number,
): Promise<void> {
  await send({
    method: 'stream.set_volume',
    params: { stream: streamRef(streamIndex), pct: Math.round(volume) },
  });
}

export async function setStreamMuteMutation(
  streamIndex: number,
  muted: boolean,
): Promise<void> {
  await send({
    method: 'stream.set_mute',
    params: { stream: streamRef(streamIndex), mute: muted },
  });
}

export async function unassignApplicationMutation(streamIndex: number): Promise<void> {
  await send({ method: 'stream.unassign', params: { stream: streamRef(streamIndex) } });
}
