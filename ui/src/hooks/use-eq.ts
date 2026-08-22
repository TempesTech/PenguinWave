import { useCallback, useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { queryClient } from '@/lib/queryClient.ts';
import { toast } from '@/hooks/use-toast.ts';
import { useDaemonEvent } from '@/hooks/use-tauri-event.ts';
import { useDebouncedMutation } from '@/hooks/use-debounced-mutation.ts';
import {
  addEqBandMutation,
  applyEqPresetMutation,
  deleteEqPresetMutation,
  eqResetSafeModeMutation,
  GET_EQ_STATE_QUERY,
  LIST_EQ_PRESETS_QUERY,
  removeEqBandMutation,
  saveEqPresetMutation,
  setEqBandMutation,
  setEqChainMutation,
  setEqPreampMutation,
} from '@/queries/eq.query';
import { EqBand, EqChainId, EqState } from '@/types/eq';

const DRAG_DEBOUNCE_MS = 50;

function errorToast(title: string) {
  return (error: unknown) =>
    toast({
      title,
      description: error instanceof Error ? error.message : String(error),
      variant: 'destructive',
    });
}

export function useEq() {
  const { data: serverState, isPending } = useQuery(GET_EQ_STATE_QUERY);
  const { data: presets } = useQuery(LIST_EQ_PRESETS_QUERY);

  // Optimistic copy: the curve must follow the pointer at 60 fps while
  // backend applies are debounced. Reconciled from eq_state_changed events.
  const [localState, setLocalState] = useState<EqState | undefined>(undefined);
  const state = localState ?? serverState;

  useEffect(() => {
    if (serverState && !localState) setLocalState(serverState);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [serverState]);

  useDaemonEvent('eq.state_changed', (event) => {
    setLocalState(event.data);
    queryClient.setQueryData(GET_EQ_STATE_QUERY.queryKey, event.data);
  });

  useDaemonEvent('eq.safe_mode', () => {
    void queryClient.invalidateQueries({ queryKey: GET_EQ_STATE_QUERY.queryKey });
    setLocalState(undefined);
  });

  const invalidate = useCallback(async () => {
    await queryClient.invalidateQueries({ queryKey: GET_EQ_STATE_QUERY.queryKey });
    setLocalState(undefined);
  }, []);

  const debouncedSetBand = useDebouncedMutation(
    ({ chain, index, band }: { chain: EqChainId; index: number; band: EqBand }) => {
      setEqBandMutation(chain, index, band).catch(errorToast('Failed to update band'));
    },
    DRAG_DEBOUNCE_MS,
  );

  /** Hot path: optimistic local update + debounced backend apply. */
  const setBand = useCallback(
    (chain: EqChainId, index: number, band: EqBand) => {
      setLocalState((prev) => {
        const current = prev?.chains[chain];
        if (!prev || !current) return prev;
        const bands = [...current.bands];
        bands[index] = band;
        return {
          ...prev,
          chains: { ...prev.chains, [chain]: { ...current, bands } },
        };
      });
      debouncedSetBand({ chain, index, band });
    },
    [debouncedSetBand],
  );

  const debouncedSetPreamp = useDebouncedMutation(
    ({ chain, gainDb }: { chain: EqChainId; gainDb: number }) => {
      setEqPreampMutation(chain, gainDb).catch(errorToast('Failed to set preamp'));
    },
    DRAG_DEBOUNCE_MS,
  );

  const setPreamp = useCallback(
    (chain: EqChainId, gainDb: number) => {
      setLocalState((prev) => {
        if (!prev) return prev;
        return {
          ...prev,
          chains: {
            ...prev.chains,
            [chain]: { ...prev.chains[chain], preamp_db: gainDb },
          },
        };
      });
      debouncedSetPreamp({ chain, gainDb });
    },
    [debouncedSetPreamp],
  );

  const setChainEnabled = useCallback((chain: EqChainId, enabled: boolean) => {
    setEqChainMutation(chain, enabled).catch(errorToast('Failed to toggle EQ'));
  }, []);

  const addBand = useCallback(
    (chain: EqChainId, band: EqBand) => {
      addEqBandMutation(chain, band).catch(errorToast('Failed to add band'));
    },
    [],
  );

  const removeBand = useCallback((chain: EqChainId, index: number) => {
    removeEqBandMutation(chain, index).catch(errorToast('Failed to remove band'));
  }, []);

  const applyPreset = useCallback((chain: EqChainId, name: string) => {
    applyEqPresetMutation(chain, name)
      .then(() => toast({ title: 'Preset applied', description: name }))
      .catch(errorToast('Failed to apply preset'));
  }, []);

  const savePreset = useCallback(async (chain: EqChainId, name: string) => {
    try {
      await saveEqPresetMutation(chain, name);
      await queryClient.invalidateQueries({ queryKey: LIST_EQ_PRESETS_QUERY.queryKey });
      toast({ title: 'Preset saved', description: name });
    } catch (error) {
      errorToast('Failed to save preset')(error);
    }
  }, []);

  const deletePreset = useCallback(async (name: string) => {
    try {
      await deleteEqPresetMutation(name);
      await queryClient.invalidateQueries({ queryKey: LIST_EQ_PRESETS_QUERY.queryKey });
      toast({ title: 'Preset deleted', description: name });
    } catch (error) {
      errorToast('Failed to delete preset')(error);
    }
  }, []);

  const resetSafeMode = useCallback(async () => {
    try {
      await eqResetSafeModeMutation();
      await invalidate();
      toast({ title: 'EQ re-enabled' });
    } catch (error) {
      errorToast('Failed to re-enable EQ')(error);
    }
  }, [invalidate]);

  return {
    state,
    isPending,
    presets: presets ?? [],
    setBand,
    setPreamp,
    setChainEnabled,
    addBand,
    removeBand,
    applyPreset,
    savePreset,
    deletePreset,
    resetSafeMode,
  };
}
