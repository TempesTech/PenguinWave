import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { chainMagnitudeDb, coefficients, logFrequencies, magnitudeDb } from '@/lib/biquad';
import { cn } from '@/lib/utils';
import {
  EqBand,
  EqChain,
  FREQ_MAX,
  FREQ_MIN,
  GAIN_MAX,
  GAIN_MIN,
  Q_MAX,
  Q_MIN,
} from '@/types/eq';

const WIDTH = 800;
const HEIGHT = 300;
const PAD = { top: 16, right: 16, bottom: 28, left: 40 };
const PLOT_W = WIDTH - PAD.left - PAD.right;
const PLOT_H = HEIGHT - PAD.top - PAD.bottom;
const CURVE_SAMPLES = 160;
const GRID_FREQS = [50, 100, 200, 500, 1000, 2000, 5000, 10000];
const GRID_GAINS = [-18, -12, -6, 0, 6, 12, 18];

const LOG_MIN = Math.log10(FREQ_MIN);
const LOG_MAX = Math.log10(FREQ_MAX);

function freqToX(freq: number): number {
  return PAD.left + ((Math.log10(freq) - LOG_MIN) / (LOG_MAX - LOG_MIN)) * PLOT_W;
}

function xToFreq(x: number): number {
  const t = Math.min(1, Math.max(0, (x - PAD.left) / PLOT_W));
  return Math.pow(10, LOG_MIN + t * (LOG_MAX - LOG_MIN));
}

function gainToY(db: number): number {
  return PAD.top + ((GAIN_MAX - db) / (GAIN_MAX - GAIN_MIN)) * PLOT_H;
}

function yToGain(y: number): number {
  const t = Math.min(1, Math.max(0, (y - PAD.top) / PLOT_H));
  return GAIN_MAX - t * (GAIN_MAX - GAIN_MIN);
}

function clampGain(db: number): number {
  return Math.min(GAIN_MAX, Math.max(GAIN_MIN, db));
}

/** 996 -> "996 Hz", 2350 -> "2.35 kHz" */
export function formatFreq(freq: number): string {
  if (freq >= 1000) {
    const k = freq / 1000;
    return `${k >= 10 ? k.toFixed(1) : k.toFixed(2)} kHz`;
  }
  return `${Math.round(freq)} Hz`;
}

function formatAxisFreq(freq: number): string {
  return freq >= 1000 ? `${freq / 1000}k` : `${freq}`;
}

/** Round a dragged frequency to a musically-sane step (3 significant digits). */
function niceFreq(freq: number): number {
  const magnitude = Math.pow(10, Math.floor(Math.log10(freq)) - 2);
  return Math.round(freq / magnitude) * magnitude;
}

function pathFor(samples: Array<[number, number]>): string {
  return samples
    .map(([x, y], i) => `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`)
    .join(' ');
}

interface EqCurveProps {
  chain: EqChain;
  selectedBand: number | null;
  onSelectBand: (index: number | null) => void;
  onBandChange: (index: number, band: EqBand) => void;
}

export function EqCurve({ chain, selectedBand, onSelectBand, onBandChange }: EqCurveProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [dragging, setDragging] = useState<number | null>(null);

  const freqs = useMemo(() => logFrequencies(CURVE_SAMPLES, FREQ_MIN, FREQ_MAX), []);

  const combined = useMemo(
    () =>
      freqs.map(
        (freq) =>
          [freqToX(freq), gainToY(clampGain(chainMagnitudeDb(chain, freq)))] as [number, number],
      ),
    [chain, freqs],
  );

  // dashed outline of just the selected band, so its contribution is legible
  const selectedPath = useMemo(() => {
    if (selectedBand === null) return null;
    const band = chain.bands[selectedBand];
    if (!band || !band.enabled) return null;
    const bq = coefficients(band);
    return pathFor(
      freqs.map(
        (freq) => [freqToX(freq), gainToY(clampGain(magnitudeDb(bq, freq)))] as [number, number],
      ),
    );
  }, [chain.bands, selectedBand, freqs]);

  const areaPath = useMemo(() => {
    const zeroY = gainToY(0);
    const first = combined[0];
    const last = combined[combined.length - 1];
    return `${pathFor(combined)} L${last[0].toFixed(1)},${zeroY} L${first[0].toFixed(1)},${zeroY} Z`;
  }, [combined]);

  const svgPoint = useCallback((clientX: number, clientY: number) => {
    const svg = svgRef.current;
    if (!svg) return { x: 0, y: 0 };
    const rect = svg.getBoundingClientRect();
    return {
      x: ((clientX - rect.left) / rect.width) * WIDTH,
      y: ((clientY - rect.top) / rect.height) * HEIGHT,
    };
  }, []);

  const handlePointerMove = useCallback(
    (event: React.PointerEvent) => {
      if (dragging === null) return;
      const band = chain.bands[dragging];
      if (!band) return;
      const { x, y } = svgPoint(event.clientX, event.clientY);
      onBandChange(dragging, {
        ...band,
        freq: Math.min(FREQ_MAX, Math.max(FREQ_MIN, niceFreq(xToFreq(x)))),
        gain_db: Math.round(yToGain(y) * 2) / 2,
      });
    },
    [dragging, chain.bands, onBandChange, svgPoint],
  );

  // Q via scroll wheel over the plot. Attached manually: React registers
  // wheel listeners as passive, so preventDefault inside onWheel cannot stop
  // the page from scrolling mid-adjustment.
  const wheelTarget = selectedBand ?? dragging;
  const wheelBand = wheelTarget !== null ? chain.bands[wheelTarget] : undefined;
  const wheelRef = useRef({ index: wheelTarget, band: wheelBand, onBandChange });
  wheelRef.current = { index: wheelTarget, band: wheelBand, onBandChange };

  useEffect(() => {
    const svg = svgRef.current;
    if (!svg) return;
    const onWheel = (event: WheelEvent) => {
      const { index, band, onBandChange } = wheelRef.current;
      if (index === null || !band) return;
      event.preventDefault();
      const factor = event.deltaY < 0 ? 1.08 : 1 / 1.08;
      onBandChange(index, {
        ...band,
        q: Math.min(Q_MAX, Math.max(Q_MIN, Number((band.q * factor).toFixed(3)))),
      });
    };
    svg.addEventListener('wheel', onWheel, { passive: false });
    return () => svg.removeEventListener('wheel', onWheel);
  }, []);

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent, index: number) => {
      const band = chain.bands[index];
      if (!band) return;
      const fine = event.shiftKey ? 0.1 : 0.5;
      let next: EqBand | null = null;
      switch (event.key) {
        case 'ArrowUp':
          next = { ...band, gain_db: clampGain(band.gain_db + fine) };
          break;
        case 'ArrowDown':
          next = { ...band, gain_db: clampGain(band.gain_db - fine) };
          break;
        case 'ArrowLeft':
          next = { ...band, freq: Math.max(FREQ_MIN, niceFreq(band.freq / (event.shiftKey ? 1.01 : 1.06))) };
          break;
        case 'ArrowRight':
          next = { ...band, freq: Math.min(FREQ_MAX, niceFreq(band.freq * (event.shiftKey ? 1.01 : 1.06))) };
          break;
        default:
          return;
      }
      event.preventDefault();
      if (next) onBandChange(index, next);
    },
    [chain.bands, onBandChange],
  );

  const activeBand =
    dragging !== null ? chain.bands[dragging] : selectedBand !== null ? chain.bands[selectedBand] : undefined;

  return (
    <svg
      ref={svgRef}
      viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
      className="w-full select-none touch-none"
      onPointerMove={handlePointerMove}
      onPointerUp={() => setDragging(null)}
      onPointerCancel={() => setDragging(null)}
      onPointerDown={(event) => {
        // click on empty plot clears the selection
        if (event.target === svgRef.current) onSelectBand(null);
      }}
      data-testid="eq-curve"
    >
      <defs>
        <linearGradient id="eq-area" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" className="[stop-color:var(--primary)]" stopOpacity="0.25" />
          <stop offset="100%" className="[stop-color:var(--primary)]" stopOpacity="0.02" />
        </linearGradient>
        <clipPath id="eq-plot">
          <rect x={PAD.left} y={PAD.top} width={PLOT_W} height={PLOT_H} />
        </clipPath>
      </defs>

      {/* grid */}
      {GRID_FREQS.map((freq) => (
        <g key={freq}>
          <line
            x1={freqToX(freq)}
            x2={freqToX(freq)}
            y1={PAD.top}
            y2={PAD.top + PLOT_H}
            className="stroke-border/60"
            strokeWidth={0.5}
          />
          <text
            x={freqToX(freq)}
            y={HEIGHT - 10}
            textAnchor="middle"
            className="fill-muted-foreground text-[10px] font-mono"
          >
            {formatAxisFreq(freq)}
          </text>
        </g>
      ))}
      {GRID_GAINS.map((db) => (
        <g key={db}>
          <line
            x1={PAD.left}
            x2={PAD.left + PLOT_W}
            y1={gainToY(db)}
            y2={gainToY(db)}
            className={db === 0 ? 'stroke-muted-foreground/50' : 'stroke-border/60'}
            strokeWidth={db === 0 ? 1 : 0.5}
          />
          <text
            x={PAD.left - 8}
            y={gainToY(db) + 3}
            textAnchor="end"
            className="fill-muted-foreground text-[10px] font-mono"
          >
            {db > 0 ? `+${db}` : db}
          </text>
        </g>
      ))}

      <g clipPath="url(#eq-plot)" className={cn(!chain.enabled && 'opacity-30')}>
        {/* filled area under the combined response */}
        <path d={areaPath} fill="url(#eq-area)" />
        {/* selected band's own response, dashed */}
        {selectedPath && (
          <path
            d={selectedPath}
            fill="none"
            className="stroke-primary/50"
            strokeWidth={1}
            strokeDasharray="4 4"
          />
        )}
        {/* combined response */}
        <path d={pathFor(combined)} fill="none" className="stroke-primary" strokeWidth={2} />
      </g>

      {/* band handles */}
      {chain.bands.map((band, index) => {
        const selected = selectedBand === index;
        const x = freqToX(band.freq);
        const y = gainToY(clampGain(band.gain_db));
        return (
          <g
            key={index}
            role="slider"
            aria-label={`Band ${index + 1}: ${formatFreq(band.freq)}`}
            aria-valuenow={band.gain_db}
            aria-valuemin={GAIN_MIN}
            aria-valuemax={GAIN_MAX}
            tabIndex={0}
            className={cn(
              'cursor-grab outline-none',
              dragging === index && 'cursor-grabbing',
              !chain.enabled && 'opacity-30',
            )}
            onKeyDown={(event) => handleKeyDown(event, index)}
            onFocus={() => onSelectBand(index)}
            onPointerDown={(event) => {
              event.stopPropagation();
              (event.currentTarget as Element).setPointerCapture?.(event.pointerId);
              onSelectBand(index);
              setDragging(index);
            }}
            data-testid={`eq-band-handle-${index}`}
          >
            <circle
              cx={x}
              cy={y}
              r={selected ? 11 : 9}
              className={cn(
                'transition-[r,fill-opacity]',
                band.enabled
                  ? selected
                    ? 'fill-primary stroke-primary-foreground'
                    : 'fill-primary/25 stroke-primary hover:fill-primary/50'
                  : 'fill-muted stroke-muted-foreground/40',
              )}
              strokeWidth={1.5}
            />
            <text
              x={x}
              y={y + 3.5}
              textAnchor="middle"
              className={cn(
                'pointer-events-none text-[10px] font-mono font-semibold',
                band.enabled && selected ? 'fill-primary-foreground' : 'fill-foreground',
              )}
            >
              {index + 1}
            </text>
          </g>
        );
      })}

      {/* live readout for the active band */}
      {activeBand && (
        <text
          x={WIDTH - PAD.right}
          y={PAD.top + 12}
          textAnchor="end"
          className="fill-foreground text-[12px] font-mono"
          data-testid="eq-readout"
        >
          {formatFreq(activeBand.freq)}
          {'   '}
          {activeBand.gain_db > 0 ? '+' : ''}
          {activeBand.gain_db.toFixed(1)} dB{'   '}Q {activeBand.q.toFixed(2)}
        </text>
      )}
    </svg>
  );
}
