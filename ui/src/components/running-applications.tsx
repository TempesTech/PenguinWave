import { useEffect, useState } from 'react';
import { Check, MicOff, Pencil, Volume2, VolumeX, X } from 'lucide-react';
import { Slider } from '@/components/ui/slider';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useMutation, useQuery } from '@tanstack/react-query';
import { PipeWireNode } from '@/types/audio_manager.ts';
import { GET_CUSTOM_VIRTUAL_SINKS_QUERY } from '@/queries/sink_manager.query.ts';
import { useToast } from '@/hooks/use-toast.ts';
import {
  GET_APPLICATION_STREAMS_QUERY,
  moveApplicationStreamToSinkMutation,
  setStreamMuteMutation,
  setStreamVolumeMutation,
} from '@/queries/application_stream.query.ts';
import { ApplicationStream } from '@/types/audio.ts';
import { useAudioState } from '@/hooks/use-audio-state.tsx';
import { useDebouncedMutation } from '@/hooks/use-debounced-mutation';
import { useStreamLabels } from '@/hooks/use-stream-labels';
import { sinkLabel } from '@/lib/sink-labels';
import { Input } from '@/components/ui/input';
import { EmptyState } from '@/components/empty-state';
import { LoadingSkeleton } from '@/components/loading-skeleton';
import { queryClient } from '@/lib/queryClient.ts';

const VOLUME_DEBOUNCE_MS = 75;

function streamsAvgVolume(streams: ApplicationStream[]): number {
  if (streams.length === 0) return 0;
  return Math.round(
    streams.reduce((sum, s) => sum + s.volume, 0) / streams.length,
  );
}

export function RunningApplications() {
  const { toast } = useToast();
  const {
    data: customSinks,
    error: customSinksError,
    isError: customSinksHasError,
  } = useQuery<PipeWireNode[]>(GET_CUSTOM_VIRTUAL_SINKS_QUERY);
  const { data: applicationStreams, isPending: streamsPending } = useQuery<
    Record<string, ApplicationStream[]>
  >(GET_APPLICATION_STREAMS_QUERY);

  const { InvalidateAudioQuery } = useAudioState();

  const [draftVolumes, setDraftVolumes] = useState<Record<string, number>>({});
  const { labels, setLabel } = useStreamLabels();
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [draftLabel, setDraftLabel] = useState('');

  const startEdit = (deviceKey: string, current: string) => {
    setEditingKey(deviceKey);
    setDraftLabel(current);
  };

  const commitEdit = () => {
    if (editingKey === null) return;
    setLabel(editingKey, draftLabel);
    setEditingKey(null);
    setDraftLabel('');
  };

  const cancelEdit = () => {
    setEditingKey(null);
    setDraftLabel('');
  };

  const { mutate: moveApplicatonStreamToSink } = useMutation({
    mutationFn: ({
      applicationId,
      sinkId,
    }: {
      applicationId: number;
      sinkId: number;
    }) => moveApplicationStreamToSinkMutation(applicationId, sinkId),
    async onSuccess() {
      await InvalidateAudioQuery();
      toast({ title: 'Success', description: 'Stream moved successfully' });
    },
    onError(error) {
      toast({
        title: 'Failed to move stream to virtual sink',
        description: error.message,
        variant: 'destructive',
      });
    },
  });

  const { mutate: muteStream } = useMutation({
    mutationFn: ({ streamId, muted }: { streamId: number; muted: boolean }) =>
      setStreamMuteMutation(streamId, muted),
    onSuccess() {
      queryClient.invalidateQueries({
        queryKey: GET_APPLICATION_STREAMS_QUERY.queryKey,
      });
    },
    onError(error) {
      toast({
        title: 'Failed to toggle mute',
        description: error.message,
        variant: 'destructive',
      });
    },
  });

  const debouncedSetVolume = useDebouncedMutation<{
    streamId: number;
    volume: number;
  }>(({ streamId, volume }) => {
    setStreamVolumeMutation(streamId, volume).catch((error) => {
      toast({
        title: 'Failed to set stream volume',
        description: error?.message ?? String(error),
        variant: 'destructive',
      });
    });
  }, VOLUME_DEBOUNCE_MS);

  useEffect(() => {
    if (customSinksHasError && customSinksError) {
      toast({
        title: 'Failed to retrieve the custom sinks',
        description: customSinksError.message,
        variant: 'destructive',
      });
    }
  }, [customSinksHasError, customSinksError]);

  useEffect(() => {
    if (!applicationStreams) return;
    setDraftVolumes((prev) => {
      const next: Record<string, number> = {};
      for (const [deviceKey, value] of Object.entries(prev)) {
        const streams = applicationStreams[deviceKey];
        if (!streams) continue;
        const realAvg = streamsAvgVolume(streams);
        if (realAvg !== value) next[deviceKey] = value;
      }
      return next;
    });
  }, [applicationStreams]);

  const changeSink = (deviceKey: string, sinkId: number) => {
    const streamsToMove = applicationStreams?.[deviceKey] || [];
    streamsToMove.forEach((stream) => {
      moveApplicatonStreamToSink({ applicationId: stream.id, sinkId });
    });
  };

  const changeVolume = (deviceKey: string, streams: ApplicationStream[], value: number) => {
    setDraftVolumes((prev) => ({ ...prev, [deviceKey]: value }));
    streams.forEach((s) => {
      debouncedSetVolume({ streamId: s.id, volume: value });
    });
  };

  const toggleMute = (streams: ApplicationStream[]) => {
    const nextMuted = !streams.some((s) => s.is_muted);
    streams.forEach((s) => {
      muteStream({ streamId: s.id, muted: nextMuted });
    });
  };

  const streamEntries = Object.entries(applicationStreams ?? {});
  const streamCount = streamEntries.reduce((n, [, s]) => n + s.length, 0);

  return (
    <section className="space-y-3">
      <div className="flex items-baseline justify-between">
        <h2 className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
          Running Applications
        </h2>
        {streamCount > 0 && (
          <span className="text-[11px] font-mono tabular-nums text-muted-foreground">
            {streamCount} {streamCount === 1 ? 'stream' : 'streams'}
          </span>
        )}
      </div>

      {streamsPending ? (
        <LoadingSkeleton variant="rows" count={3} />
      ) : streamEntries.length === 0 ? (
        <div className="rounded-xl border border-dashed border-border">
          <EmptyState
            icon={MicOff}
            title="No applications playing audio"
            description="Start a media app (Spotify, Discord, a game) and it will appear here."
          />
        </div>
      ) : (
        <div className="rounded-xl border border-border bg-card divide-y divide-border overflow-hidden">
          {streamEntries.map(([deviceKey, streams]) => {
            const primaryStream = streams[0];
            if (!primaryStream) return null;

            const allStreamsOnSameSink = streams.every(
              (s) => s.assigned_sink_id === primaryStream.assigned_sink_id,
            );
            const currentSinkId = allStreamsOnSameSink
              ? primaryStream.assigned_sink_id
              : null;

            const anyMuted = streams.some((s) => s.is_muted);
            const realAvg = streamsAvgVolume(streams);
            const sliderValue = draftVolumes[deviceKey] ?? realAvg;

            return (
              <div
                key={deviceKey}
                className="grid grid-cols-12 gap-3 items-center px-4 py-3 group/row"
              >
                <div className="col-span-4 flex items-center gap-3 min-w-0">
                  <div className="w-9 h-9 bg-muted rounded-lg flex items-center justify-center text-sm overflow-hidden shrink-0">
                    {primaryStream.icon_path ? (
                      <img
                        src={primaryStream.icon_path}
                        alt=""
                        className="w-full h-full object-contain"
                      />
                    ) : (
                      <span className="opacity-60">🎵</span>
                    )}
                  </div>
                  <div className="min-w-0 flex-1">
                    {editingKey === deviceKey ? (
                      <div className="flex items-center gap-1">
                        <Input
                          autoFocus
                          value={draftLabel}
                          onChange={(e) => setDraftLabel(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') commitEdit();
                            else if (e.key === 'Escape') cancelEdit();
                          }}
                          className="h-7 text-sm"
                          placeholder={primaryStream.name}
                        />
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={commitEdit}
                          className="h-7 w-7 p-0"
                          aria-label="Save label"
                        >
                          <Check className="h-3 w-3" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={cancelEdit}
                          className="h-7 w-7 p-0"
                          aria-label="Cancel"
                        >
                          <X className="h-3 w-3" />
                        </Button>
                      </div>
                    ) : (
                      <>
                        <div className="flex items-center gap-1">
                          <div className="text-sm text-foreground font-medium truncate">
                            {labels[deviceKey] ?? primaryStream.name}
                          </div>
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() =>
                              startEdit(
                                deviceKey,
                                labels[deviceKey] ?? primaryStream.name,
                              )
                            }
                            className="h-5 w-5 p-0 opacity-0 group-hover/row:opacity-100 transition-opacity text-muted-foreground hover:text-foreground"
                            aria-label="Rename"
                          >
                            <Pencil className="h-3 w-3" />
                          </Button>
                        </div>
                        <div className="text-[11px] text-muted-foreground truncate">
                          {labels[deviceKey]
                            ? primaryStream.name
                            : streams.length > 1
                            ? `${streams.length} streams`
                            : 'Audio stream'}
                        </div>
                      </>
                    )}
                  </div>
                </div>

                <div className="col-span-3">
                  <Select
                    value={
                      currentSinkId
                        ? customSinks
                            ?.find((x) => x.id === currentSinkId)
                            ?.id.toString() ?? 'not-assigned'
                        : 'mixed'
                    }
                    onValueChange={(value) => changeSink(deviceKey, Number(value))}
                  >
                    <SelectTrigger className="h-8 bg-background border-border text-foreground text-xs">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent className="bg-background border-border">
                      <SelectItem value="not-assigned" className="text-foreground hover:bg-accent">
                        Not assigned
                      </SelectItem>
                      {!allStreamsOnSameSink && (
                        <SelectItem value="mixed" className="text-foreground hover:bg-accent" disabled>
                          Mixed assignments
                        </SelectItem>
                      )}
                      {customSinks?.map((sink) => (
                        <SelectItem
                          key={sink.id}
                          value={sink.id.toString()}
                          className="text-foreground hover:bg-accent"
                        >
                          {sinkLabel(sink.name, sink.description)}
                        </SelectItem>
                      )) || []}
                    </SelectContent>
                  </Select>
                </div>

                <div className="col-span-4 flex items-center gap-3">
                  <Slider
                    value={[anyMuted ? 0 : sliderValue]}
                    onValueChange={(value) =>
                      changeVolume(deviceKey, streams, value[0])
                    }
                    max={100}
                    min={0}
                    step={1}
                    className="flex-1"
                    disabled={anyMuted}
                  />
                  <span
                    className={
                      'text-[11px] font-mono tabular-nums w-12 text-right ' +
                      (anyMuted ? 'text-destructive' : 'text-muted-foreground')
                    }
                  >
                    {anyMuted ? 'muted' : `${sliderValue}%`}
                  </span>
                </div>

                <div className="col-span-1 flex items-center justify-end">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => toggleMute(streams)}
                    className="h-8 w-8 p-0 text-muted-foreground hover:text-foreground hover:bg-accent"
                    aria-label={anyMuted ? 'Unmute' : 'Mute'}
                  >
                    {anyMuted ? (
                      <VolumeX className="h-4 w-4" />
                    ) : (
                      <Volume2 className="h-4 w-4" />
                    )}
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
