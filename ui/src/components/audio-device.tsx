import { useEffect, useState } from 'react';
import { Boxes, RefreshCw, Trash2 } from 'lucide-react';
import { EmptyState } from '@/components/empty-state';
import { LoadingSkeleton } from '@/components/loading-skeleton';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Slider } from '@/components/ui/slider';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AudioDevice, PipeWireNode } from '@/types/audio_manager';
import { toast } from '@/hooks/use-toast';
import {
  GET_CUSTOM_VIRTUAL_SINKS_QUERY,
  GET_DEFAULT_SINK_QUERY,
  GET_OUTPUT_DEVICES_QUERY,
  routeSinkToDeviceMutation,
  setSinkVolumeMutation,
  sinkRouteQuery,
} from '@/queries/sink_manager.query';
import { useDebouncedMutation } from '@/hooks/use-debounced-mutation';
import { sinkLabel } from '@/lib/sink-labels';

const VOLUME_DEBOUNCE_MS = 75;

function SinkRouter({
  sink,
  outputDevices,
  virtualSinkNames,
}: {
  sink: PipeWireNode;
  outputDevices: AudioDevice[];
  virtualSinkNames: Set<string>;
}) {
  const queryClient = useQueryClient();
  const { data: currentRoute } = useQuery(sinkRouteQuery(sink.name));

  const { mutate: reroute, isPending } = useMutation({
    mutationFn: ({ target }: { target: string }) =>
      routeSinkToDeviceMutation(sink.name, target),
    onSuccess(_, { target }) {
      queryClient.invalidateQueries({ queryKey: sinkRouteQuery(sink.name).queryKey });
      toast({ title: `${sink.name} → ${target}` });
    },
    onError(e) {
      toast({ title: 'Routing failed', description: e.message, variant: 'destructive' });
    },
  });

  const physicalDevices = outputDevices.filter((d) => !virtualSinkNames.has(d.name));

  return (
    <div className="flex items-center gap-3">
      <span className="text-xs text-muted-foreground w-12 shrink-0">Output</span>
      <Select
        value={currentRoute ?? ''}
        onValueChange={(target) => reroute({ target })}
        disabled={isPending}
      >
        <SelectTrigger className="h-7 text-xs bg-secondary border-border flex-1">
          <SelectValue placeholder="Select output…" />
        </SelectTrigger>
        <SelectContent className="bg-background border-border">
          {physicalDevices.map((d) => (
            <SelectItem key={d.name} value={d.name} className="text-xs text-foreground hover:bg-accent">
              {d.description || d.name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}

interface AudioDeviceCardProps {
  sink: PipeWireNode;
  isDefault: boolean;
  draftVolume: number | undefined;
  onVolumeChange: (value: number) => void;
  onDelete?: (moduleId: number) => void;
  outputDevices: AudioDevice[];
  virtualSinkNames: Set<string>;
}

function AudioDeviceCard({
  sink,
  isDefault,
  draftVolume,
  onVolumeChange,
  onDelete,
  outputDevices,
  virtualSinkNames,
}: AudioDeviceCardProps) {
  const displayVolume = draftVolume ?? sink.volume;

  return (
    <Card className="bg-card border-border">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 min-w-0">
            <CardTitle className="text-foreground text-sm truncate">
              {sinkLabel(sink.name, sink.description)}
            </CardTitle>
            {isDefault && (
              <span className="text-[10px] uppercase tracking-wide px-2 py-0.5 rounded bg-primary/10 text-primary">
                Default
              </span>
            )}
          </div>
          {onDelete && sink.module_id != null && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onDelete(sink.module_id!)}
              className="text-muted-foreground hover:text-destructive hover:bg-destructive/10 p-1 h-6 w-6"
              aria-label={`Delete ${sink.name}`}
            >
              <Trash2 className="h-3 w-3" />
            </Button>
          )}
        </div>
        {sink.description && (
          <div className="text-xs text-muted-foreground truncate">
            {sink.description}
          </div>
        )}
      </CardHeader>
      <CardContent className="pt-0">
        <div className="space-y-3">
          <div className="flex items-center gap-3">
            <span className="text-xs text-muted-foreground w-12">Volume</span>
            <Slider
              value={[displayVolume]}
              onValueChange={(value) => onVolumeChange(value[0])}
              max={100}
              min={0}
              step={1}
              className="flex-1"
            />
            <span className="text-xs font-mono tabular-nums text-muted-foreground w-10 text-right">
              {displayVolume}%
            </span>
          </div>
          <SinkRouter
            sink={sink}
            outputDevices={outputDevices}
            virtualSinkNames={virtualSinkNames}
          />
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>Module · Sink</span>
            <span className="font-mono">
              {sink.module_id ?? '-'} · {sink.id}
            </span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

interface AudioDevicesProps {
  customSinks?: PipeWireNode[];
  isLoading?: boolean;
  onRefresh?: () => void;
  onDeleteSink?: (id: number) => void;
}

export function AudioDevices({
  customSinks = [],
  isLoading = false,
  onRefresh,
  onDeleteSink,
}: AudioDevicesProps) {
  const queryClient = useQueryClient();
  const { data: defaultSinkName } = useQuery(GET_DEFAULT_SINK_QUERY);
  const { data: outputDevices = [] } = useQuery(GET_OUTPUT_DEVICES_QUERY);
  const virtualSinkNames = new Set(customSinks.map((s) => s.name));
  const [draftVolumes, setDraftVolumes] = useState<Record<number, number>>({});

  const { mutate: persistVolume } = useMutation({
    mutationFn: ({ sinkId, volume }: { sinkId: number; volume: number }) =>
      setSinkVolumeMutation(sinkId, volume),
    onError(error) {
      toast({
        title: 'Failed to set sink volume',
        description: error.message,
        variant: 'destructive',
      });
    },
    onSettled() {
      queryClient.invalidateQueries({
        queryKey: GET_CUSTOM_VIRTUAL_SINKS_QUERY.queryKey,
      });
    },
  });

  const debouncedPersist = useDebouncedMutation<{ sinkId: number; volume: number }>(
    (vars) => persistVolume(vars),
    VOLUME_DEBOUNCE_MS,
  );

  // Drop drafts whose backend value caught up
  useEffect(() => {
    setDraftVolumes((prev) => {
      const next: Record<number, number> = {};
      for (const [id, value] of Object.entries(prev)) {
        const sinkId = Number(id);
        const sink = customSinks.find((s) => s.id === sinkId);
        if (sink && sink.volume !== value) next[sinkId] = value;
      }
      return next;
    });
  }, [customSinks]);

  const handleVolumeChange = (sinkId: number, volume: number) => {
    setDraftVolumes((prev) => ({ ...prev, [sinkId]: volume }));
    debouncedPersist({ sinkId, volume });
  };

  return (
    <section className="space-y-3">
      <div className="flex items-baseline justify-between">
        <h2 className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
          Virtual Audio Sinks
        </h2>
        <div className="flex items-center gap-2">
          {customSinks.length > 0 && (
            <span className="text-[11px] font-mono tabular-nums text-muted-foreground">
              {customSinks.length}
            </span>
          )}
          {onRefresh && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onRefresh}
              disabled={isLoading}
              className="h-7 w-7 p-0 text-muted-foreground hover:text-foreground"
              aria-label="Refresh sinks"
            >
              <RefreshCw
                className={`h-3.5 w-3.5 ${isLoading ? 'animate-spin' : ''}`}
              />
            </Button>
          )}
        </div>
      </div>

      {isLoading ? (
        <LoadingSkeleton variant="cards" count={3} />
      ) : customSinks.length > 0 ? (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {customSinks.map((sink) => (
            <AudioDeviceCard
              key={sink.id}
              sink={sink}
              isDefault={defaultSinkName === sink.name}
              draftVolume={draftVolumes[sink.id]}
              onVolumeChange={(v) => handleVolumeChange(sink.id, v)}
              onDelete={onDeleteSink}
              outputDevices={outputDevices}
              virtualSinkNames={virtualSinkNames}
            />
          ))}
        </div>
      ) : (
        <div className="rounded-xl border border-dashed border-border">
          <EmptyState
            icon={Boxes}
            title="No virtual sinks yet"
            description='Click "Create Sink" above to start routing audio.'
          />
        </div>
      )}
    </section>
  );
}
