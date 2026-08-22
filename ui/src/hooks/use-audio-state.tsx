import { useCallback } from 'react';
import {useMutation, useQuery} from "@tanstack/react-query";
import {GET_AUDIO_CATEGORIES_QUERY} from "@/queries/audio.query.ts";
import {
  GET_APPLICATION_STREAMS_QUERY,
  moveApplicationStreamToSinkMutation,
  unassignApplicationMutation
} from "@/queries/application_stream.query.ts";
import {GET_CUSTOM_VIRTUAL_SINKS_QUERY, setSinkVolumeMutation} from "@/queries/sink_manager.query.ts";
import {queryClient} from "@/lib/queryClient.ts";
import {toast} from "@/hooks/use-toast.ts";
import {consumeOrigin, rememberOrigin} from "@/lib/stream-origins";
import {useDaemonEvent} from "@/hooks/use-tauri-event.ts";

export function useAudioState() {
  const {data: categoriesNew, isPending: categoriesPending} = useQuery(GET_AUDIO_CATEGORIES_QUERY);
  const {data: applicationStreams} = useQuery(GET_APPLICATION_STREAMS_QUERY);
  const {data: customSinks} = useQuery(GET_CUSTOM_VIRTUAL_SINKS_QUERY);

  // Backend debounces `pactl subscribe` bursts into one signal -> refetch the lists.
  useDaemonEvent('graph.changed', () => {
    void InvalidateAudioQuery();
  });

  const { mutate: setSinkVolume } = useMutation({
    mutationFn: ({
                   sinkId,
                   volume
                 }: {
      sinkId: number,
      volume: number
    }) => setSinkVolumeMutation(sinkId, volume),
    async onSuccess() {
      await InvalidateAudioQuery();
      toast({
        title: 'Volume Updated',
        description: `Sink volume set successfully`,
      });
    },
    onError(error) {
      toast({
        title: 'Failed to set volume',
        description: error.message,
        variant: 'destructive',
      });
    },
  });

  const updateCategoryVolume = useCallback((categoryId: string, volume: number) => {
    setSinkVolume({ sinkId: parseInt(categoryId), volume });
  }, [setSinkVolume]);

  const { mutate: moveApplicatonStreamToSink } = useMutation({
    mutationFn: ({
                   applicationId,
                   sinkId
                 }: {
      applicationId: number,
      sinkId: number
    }) => moveApplicationStreamToSinkMutation(applicationId, sinkId),
    async onSuccess() {
      toast({
        title: 'Success',
        description: `Stream moved successfully`,
      });
    },
    onError(error) {
      toast({
        title: 'Failed to move stream to virtual sink',
        description: error.message,
        variant: 'destructive',
      });
    },
  })

  const moveApplication = useCallback((appId: string, fromCategoryId: string, toCategoryId: string) => {
    void fromCategoryId;
    const applicationInCategory = applicationStreams?.[appId];
    if (!applicationInCategory) {
      return;
    }

    const isUnassign = toCategoryId === 'others';
    const customSinkIds = new Set((customSinks ?? []).map(s => s.id));

    const work = applicationInCategory.map(app => {
      if (isUnassign) {
        const origin = consumeOrigin(app.id);
        // restore to original non-custom sink if we tracked one, else default
        if (origin !== undefined) {
          return new Promise<void>((resolve) => {
            moveApplicatonStreamToSink(
              { applicationId: app.id, sinkId: origin },
              { onSettled: () => resolve() },
            );
          });
        }
        return unassignApplicationMutation(app.id).catch(error => {
          toast({
            title: 'Failed to unassign stream',
            description: error?.message ?? String(error),
            variant: 'destructive',
          });
        });
      }

      // moving into a custom sink: remember pre-custom origin once
      const targetId = parseInt(toCategoryId);
      if (!customSinkIds.has(app.assigned_sink_id)) {
        rememberOrigin(app.id, app.assigned_sink_id);
      }

      return new Promise<void>((resolve) => {
        moveApplicatonStreamToSink(
          { applicationId: app.id, sinkId: targetId },
          { onSettled: () => resolve() },
        );
      });
    });

    Promise.all(work).then(async () => {
      await InvalidateAudioQuery();
    });
  }, [applicationStreams, customSinks, moveApplicatonStreamToSink]);

  async function InvalidateAudioQuery() {
    await queryClient.invalidateQueries({queryKey: GET_AUDIO_CATEGORIES_QUERY.queryKey});
    await queryClient.invalidateQueries({queryKey: GET_APPLICATION_STREAMS_QUERY.queryKey});
  }

  return {
    updateCategoryVolume,
    moveApplication,
    categoriesNew,
    categoriesPending,
    InvalidateAudioQuery
  };
}
