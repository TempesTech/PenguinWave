import { Query } from './query';
import { Application, AudioCategory } from '@/types/audio.ts';
import { request } from '@/lib/daemon';
import { sinkLabel } from '@/lib/sink-labels';
import { streamsByApp } from '@/lib/adapters';

const QUERY_KEYS = {
  GET_AUDIO_CATEGORIES: 'getAudioCategories',
} as const;

export const GET_AUDIO_CATEGORIES_QUERY: Query<AudioCategory[]> = {
  queryKey: [QUERY_KEYS.GET_AUDIO_CATEGORIES],
  queryFn: async () => {
    const [streams, sinks] = await Promise.all([
      request({ method: 'stream.list' }, 'streams'),
      request({ method: 'sink.list_custom' }, 'sinks'),
    ]);
    const audioStreams = await streamsByApp(streams, sinks);

    const others: AudioCategory = {
      id: 'others',
      name: 'Unassigned',
      volume: 0,
      applications: Object.entries(audioStreams)
        .filter(([, grouped]) =>
          grouped.every((stream) => !sinks.some((sink) => sink.id === stream.assigned_sink_id)),
        )
        .map(([key, grouped]) => {
          const sample = grouped[0];
          return {
            id: key,
            name: key,
            description: '',
            icon: sample?.icon ?? '',
            icon_path: sample?.icon_path,
            category: 'others' as const,
          };
        }),
    };

    const categories: AudioCategory[] = sinks.map((sink) => {
      const streamsForSink = Object.values(audioStreams)
        .flat()
        .filter((stream) => stream.assigned_sink_id === sink.id);

      const uniqueApplications = new Map<string, Application>();
      streamsForSink.forEach((stream) => {
        if (!uniqueApplications.has(stream.name)) {
          uniqueApplications.set(stream.name, {
            id: stream.name,
            name: stream.name,
            description: '',
            icon: stream.icon || '',
            icon_path: stream.icon_path,
            category: 'others',
          });
        }
      });

      return {
        id: sink.id.toString(),
        name: sinkLabel(sink.name, sink.description),
        volume: sink.volume,
        applications: Array.from(uniqueApplications.values()),
      };
    });

    return [others, ...categories];
  },
};
