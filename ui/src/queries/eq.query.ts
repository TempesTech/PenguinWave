import { Query } from '@/queries/query.ts';
import { EqBand, EqChainId, EqPresetMeta, EqState } from '@/types/eq';
import { request, send } from '@/lib/daemon';

//#region Queries
const QUERY_KEYS = {
  GET_EQ_STATE: 'getEqState',
  LIST_EQ_PRESETS: 'listEqPresets',
} as const;

export const GET_EQ_STATE_QUERY: Query<EqState> = {
  queryKey: [QUERY_KEYS.GET_EQ_STATE],
  queryFn: async () => request({ method: 'eq.get_state' }, 'eq_state') as Promise<EqState>,
};

export const LIST_EQ_PRESETS_QUERY: Query<EqPresetMeta[]> = {
  queryKey: [QUERY_KEYS.LIST_EQ_PRESETS],
  queryFn: async () => request({ method: 'eq.list_presets' }, 'eq_presets'),
};
//#endregion

//#region Mutations
export async function setEqChainMutation(chain: EqChainId, enabled: boolean): Promise<void> {
  await send({ method: 'eq.set_chain_enabled', params: { chain, enabled } });
}

export async function setEqBandMutation(
  chain: EqChainId,
  index: number,
  band: EqBand,
): Promise<void> {
  await send({ method: 'eq.set_band', params: { chain, index, band } });
}

export async function addEqBandMutation(chain: EqChainId, band: EqBand): Promise<void> {
  await send({ method: 'eq.add_band', params: { chain, band } });
}

export async function removeEqBandMutation(chain: EqChainId, index: number): Promise<void> {
  await send({ method: 'eq.remove_band', params: { chain, index } });
}

export async function setEqPreampMutation(chain: EqChainId, gainDb: number): Promise<void> {
  await send({ method: 'eq.set_preamp', params: { chain, preamp_db: gainDb } });
}

export async function applyEqPresetMutation(chain: EqChainId, name: string): Promise<void> {
  await send({ method: 'eq.apply_preset', params: { chain, name } });
}

export async function saveEqPresetMutation(chain: EqChainId, name: string): Promise<void> {
  await send({ method: 'eq.save_preset', params: { chain, name } });
}

export async function deleteEqPresetMutation(name: string): Promise<void> {
  await send({ method: 'eq.delete_preset', params: { name } });
}

export async function eqResetSafeModeMutation(): Promise<void> {
  await send({ method: 'eq.reset_safe_mode' });
}
//#endregion
