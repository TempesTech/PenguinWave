// Shapes come from penguinwave-proto; only the UI's own labels live here.
//
// Hand-maintained twins of wire types drift, and the drift surfaces as a
// runtime `undefined` rather than a compile error.

export type { EqBand } from '@proto/EqBand';
export type { EqChain } from '@proto/EqChain';
export type { EqChainId } from '@proto/EqChainId';
export type { EqPresetMeta } from '@proto/EqPresetMeta';
export type { EqState } from '@proto/EqState';
export type { FilterType } from '@proto/FilterType';

import type { EqBand } from '@proto/EqBand';
import type { EqChainId } from '@proto/EqChainId';
import type { FilterType } from '@proto/FilterType';

export const MAX_BANDS = 16;
export const FREQ_MIN = 20;
export const FREQ_MAX = 20000;
export const GAIN_MIN = -24;
export const GAIN_MAX = 24;
export const Q_MIN = 0.1;
export const Q_MAX = 10;

export const FILTER_TYPE_LABELS: Record<FilterType, string> = {
  peaking: 'Peaking',
  low_shelf: 'Low Shelf',
  high_shelf: 'High Shelf',
  low_pass: 'Low Pass',
  high_pass: 'High Pass',
  notch: 'Notch',
};

export const CHAIN_LABELS: Record<EqChainId, string> = {
  game: 'Game',
  chat: 'Chat',
};

export function defaultBand(freq = 1000): EqBand {
  return {
    freq,
    gain_db: 0,
    q: 1.1,
    filter_type: 'peaking',
    enabled: true,
  };
}
