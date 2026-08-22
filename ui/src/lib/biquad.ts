// Biquad magnitude-response math — TS twin of src-tauri/src/eq/biquad.rs
// (RBJ Audio-EQ-Cookbook). Used only for rendering the frequency-response
// curve; the audible coefficients are computed in Rust.

import { EqBand, EqChain } from '@/types/eq';

export const SAMPLE_RATE = 48000;

export interface Biquad {
  b0: number;
  b1: number;
  b2: number;
  a1: number;
  a2: number;
}

export const IDENTITY: Biquad = { b0: 1, b1: 0, b2: 0, a1: 0, a2: 0 };

export function coefficients(band: EqBand, sampleRate = SAMPLE_RATE): Biquad {
  if (!band.enabled) return IDENTITY;

  const a = Math.pow(10, band.gain_db / 40);
  const w0 = (2 * Math.PI * band.freq) / sampleRate;
  const sinW0 = Math.sin(w0);
  const cosW0 = Math.cos(w0);
  const alpha = sinW0 / (2 * band.q);

  let b0: number, b1: number, b2: number, a0: number, a1: number, a2: number;

  switch (band.filter_type) {
    case 'peaking':
      b0 = 1 + alpha * a;
      b1 = -2 * cosW0;
      b2 = 1 - alpha * a;
      a0 = 1 + alpha / a;
      a1 = -2 * cosW0;
      a2 = 1 - alpha / a;
      break;
    case 'low_shelf': {
      const s = 2 * Math.sqrt(a) * alpha;
      b0 = a * (a + 1 - (a - 1) * cosW0 + s);
      b1 = 2 * a * (a - 1 - (a + 1) * cosW0);
      b2 = a * (a + 1 - (a - 1) * cosW0 - s);
      a0 = a + 1 + (a - 1) * cosW0 + s;
      a1 = -2 * (a - 1 + (a + 1) * cosW0);
      a2 = a + 1 + (a - 1) * cosW0 - s;
      break;
    }
    case 'high_shelf': {
      const s = 2 * Math.sqrt(a) * alpha;
      b0 = a * (a + 1 + (a - 1) * cosW0 + s);
      b1 = -2 * a * (a - 1 + (a + 1) * cosW0);
      b2 = a * (a + 1 + (a - 1) * cosW0 - s);
      a0 = a + 1 - (a - 1) * cosW0 + s;
      a1 = 2 * (a - 1 - (a + 1) * cosW0);
      a2 = a + 1 - (a - 1) * cosW0 - s;
      break;
    }
    case 'low_pass':
      b0 = (1 - cosW0) / 2;
      b1 = 1 - cosW0;
      b2 = (1 - cosW0) / 2;
      a0 = 1 + alpha;
      a1 = -2 * cosW0;
      a2 = 1 - alpha;
      break;
    case 'high_pass':
      b0 = (1 + cosW0) / 2;
      b1 = -(1 + cosW0);
      b2 = (1 + cosW0) / 2;
      a0 = 1 + alpha;
      a1 = -2 * cosW0;
      a2 = 1 - alpha;
      break;
    case 'notch':
      b0 = 1;
      b1 = -2 * cosW0;
      b2 = 1;
      a0 = 1 + alpha;
      a1 = -2 * cosW0;
      a2 = 1 - alpha;
      break;
  }

  return { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0 };
}

export function magnitudeDb(bq: Biquad, freq: number, sampleRate = SAMPLE_RATE): number {
  const w = (2 * Math.PI * freq) / sampleRate;
  const cosW = Math.cos(w);
  const sinW = Math.sin(w);
  const cos2W = Math.cos(2 * w);
  const sin2W = Math.sin(2 * w);

  const numRe = bq.b0 + bq.b1 * cosW + bq.b2 * cos2W;
  const numIm = -(bq.b1 * sinW + bq.b2 * sin2W);
  const denRe = 1 + bq.a1 * cosW + bq.a2 * cos2W;
  const denIm = -(bq.a1 * sinW + bq.a2 * sin2W);

  const num = numRe * numRe + numIm * numIm;
  const den = denRe * denRe + denIm * denIm;
  return 10 * Math.log10(num / den);
}

/// Combined chain response (preamp + all enabled bands) at one frequency.
export function chainMagnitudeDb(chain: EqChain, freq: number): number {
  if (!chain.enabled) return 0;
  let db = chain.preamp_db;
  for (const band of chain.bands) {
    db += magnitudeDb(coefficients(band), freq);
  }
  return db;
}

/// Log-spaced frequency samples for curve rendering.
export function logFrequencies(count: number, min = 20, max = 20000): number[] {
  const logMin = Math.log10(min);
  const logMax = Math.log10(max);
  return Array.from({ length: count }, (_, i) =>
    Math.pow(10, logMin + ((logMax - logMin) * i) / (count - 1)),
  );
}
