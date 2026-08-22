import { useState } from 'react';
import { Gamepad2, Headphones, MessageCircle } from 'lucide-react';
import { useSelectedDevice } from '@/hooks/use-selected-device';
import { useDaemonEvent } from '@/hooks/use-tauri-event';
import { useDebouncedMutation } from '@/hooks/use-debounced-mutation';
import { chatmixSetManualMutation } from '@/queries/device.query';
import { Slider } from '@/components/ui/slider';
import { Skeleton } from '@/components/ui/skeleton';

const CHATMIX_MAX = 128;
const CHATMIX_DEFAULT = 64;
const MANUAL_DEBOUNCE_MS = 75;

function formatHex(value: number): string {
  return value.toString(16).padStart(4, '0').toUpperCase();
}

function splitFromPosition(position: number): { game: number; chat: number } {
  const clamped = Math.max(0, Math.min(position, CHATMIX_MAX));
  const game = 100 - Math.round((clamped * 100) / CHATMIX_MAX);
  return { game, chat: 100 - game };
}

export function HeadsetPanel() {
  const { selectedDevice, isPending } = useSelectedDevice();
  const [chatmixValue, setChatmixValue] = useState(CHATMIX_DEFAULT);
  const hasHeadset = !!selectedDevice?.is_connected;

  useDaemonEvent('chatmix.changed', (event) => {
    setChatmixValue(event.data.value);
  });

  const debouncedManualMutate = useDebouncedMutation<number>((position) => {
    chatmixSetManualMutation(position).catch(() => {
      /* error surfaced via Tauri channel; no slider toast spam */
    });
  }, MANUAL_DEBOUNCE_MS);

  const handleSliderChange = (value: number[]) => {
    const next = value[0];
    setChatmixValue(next);
    if (!hasHeadset) debouncedManualMutate(next);
  };

  const { game, chat } = splitFromPosition(chatmixValue);

  if (isPending && !selectedDevice) {
    return (
      <section className="rounded-xl border border-border bg-card p-6 space-y-6">
        <div className="flex items-center gap-3">
          <Headphones className="h-5 w-5 text-muted-foreground" />
          <Skeleton className="h-4 w-48" />
        </div>
        <Skeleton className="h-2 w-full" />
      </section>
    );
  }

  return (
    <section className="rounded-xl border border-border bg-card p-6 space-y-6">
      <header className="flex items-start justify-between gap-4">
        <div className="flex items-center gap-3 min-w-0">
          <div className="rounded-lg bg-primary/10 p-2">
            <Headphones className="h-5 w-5 text-primary" />
          </div>
          <div className="min-w-0">
            <div className="text-sm font-medium text-foreground truncate">
              {selectedDevice?.name ?? 'No headset selected'}
            </div>
            {selectedDevice && (
              <div className="text-[11px] font-mono tabular-nums text-muted-foreground">
                VID {formatHex(selectedDevice.vendor_id)} · PID{' '}
                {formatHex(selectedDevice.product_id)}
              </div>
            )}
          </div>
        </div>
        {selectedDevice && (
          <span
            className={
              'inline-flex items-center gap-1.5 text-[10px] uppercase tracking-wider px-2 py-1 rounded ' +
              (selectedDevice.is_connected
                ? 'bg-primary/15 text-primary'
                : 'bg-destructive/15 text-destructive')
            }
          >
            <span
              className={
                'h-1.5 w-1.5 rounded-full ' +
                (selectedDevice.is_connected
                  ? 'bg-primary animate-pulse'
                  : 'bg-destructive')
              }
            />
            {selectedDevice.is_connected ? 'Live' : 'Offline'}
          </span>
        )}
      </header>

      <div className="space-y-3">
        <div className="flex items-baseline justify-between">
          <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
            ChatMix
          </div>
          <div className="flex items-baseline gap-1 font-mono tabular-nums">
            <span className="text-3xl text-foreground">{chatmixValue}</span>
            <span className="text-xs text-muted-foreground">/ {CHATMIX_MAX}</span>
          </div>
        </div>

        <div className="flex items-center gap-4">
          <Gamepad2 className="h-4 w-4 text-muted-foreground shrink-0" />
          <Slider
            value={[chatmixValue]}
            onValueChange={handleSliderChange}
            disabled={hasHeadset}
            max={CHATMIX_MAX}
            min={0}
            step={1}
            className="flex-1"
          />
          <MessageCircle className="h-4 w-4 text-muted-foreground shrink-0" />
        </div>

        <div className="grid grid-cols-2 gap-2 pt-2">
          <SplitBar label="Game" percent={game} icon={Gamepad2} />
          <SplitBar label="Chat" percent={chat} icon={MessageCircle} align="right" />
        </div>

        {hasHeadset ? (
          <p className="text-[11px] text-muted-foreground">
            Driven by the headset wheel. Slider is read-only while connected.
          </p>
        ) : (
          <p className="text-[11px] text-muted-foreground">
            Manual override active. No headset connected.
          </p>
        )}
      </div>
    </section>
  );
}

interface SplitBarProps {
  label: string;
  percent: number;
  icon: typeof Gamepad2;
  align?: 'left' | 'right';
}

function SplitBar({ label, percent, icon: Icon, align = 'left' }: SplitBarProps) {
  return (
    <div className={'space-y-1 ' + (align === 'right' ? 'text-right' : '')}>
      <div
        className={
          'flex items-center gap-1.5 text-[11px] text-muted-foreground ' +
          (align === 'right' ? 'justify-end' : '')
        }
      >
        {align === 'left' && <Icon className="h-3 w-3" />}
        <span className="uppercase tracking-wider">{label}</span>
        <span className="font-mono tabular-nums text-foreground">{percent}%</span>
        {align === 'right' && <Icon className="h-3 w-3" />}
      </div>
      <div className="h-1 rounded-full bg-muted overflow-hidden">
        <div
          className={
            'h-full bg-primary transition-[width] duration-150 ' +
            (align === 'right' ? 'ml-auto' : '')
          }
          style={{ width: `${percent}%` }}
        />
      </div>
    </div>
  );
}
