import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Gamepad2, Headphones, MessageCircle } from 'lucide-react';
import { useSelectedDevice } from '@/hooks/use-selected-device';
import { useDaemonEvent } from '@/hooks/use-tauri-event';
import { useDebouncedMutation } from '@/hooks/use-debounced-mutation';
import { chatmixSetManualMutation, GET_CHATMIX_QUERY } from '@/queries/device.query';
import { Button } from '@/components/ui/button';
import { Slider } from '@/components/ui/slider';
import { Skeleton } from '@/components/ui/skeleton';

// The protocol carries the balance as a game share: 0 is all chat, 100 all
// game. The raw 0..128 wheel scale stops at the HID driver.
const CHATMIX_MAX = 100;
const CHATMIX_DEFAULT = 50;
const MANUAL_DEBOUNCE_MS = 75;

function formatHex(value: number): string {
  return value.toString(16).padStart(4, '0').toUpperCase();
}

function splitFromPosition(position: number): { game: number; chat: number } {
  const game = Math.max(0, Math.min(position, CHATMIX_MAX));
  return { game, chat: 100 - game };
}

export function HeadsetPanel() {
  const { selectedDevice, isPending } = useSelectedDevice();
  const { data: chatmix } = useQuery(GET_CHATMIX_QUERY);
  const [chatmixValue, setChatmixValue] = useState(CHATMIX_DEFAULT);
  const [manual, setManual] = useState(false);
  const hasHeadset = !!selectedDevice?.is_connected;

  // Seed from the daemon rather than a guess: the balance is live state that
  // outlives this window.
  useEffect(() => {
    if (chatmix) {
      setChatmixValue(chatmix.value);
      setManual(chatmix.manual);
    }
  }, [chatmix]);

  useDaemonEvent('chatmix.changed', (event) => {
    setChatmixValue(event.data.value);
    setManual(event.data.manual);
  });

  const debouncedManualMutate = useDebouncedMutation<number>((position) => {
    chatmixSetManualMutation(position).catch(() => {
      /* error surfaced via Tauri channel; no slider toast spam */
    });
  }, MANUAL_DEBOUNCE_MS);

  // Dragging pins the balance. The wheel keeps control until then, and "Follow
  // wheel" hands it back -- the old panel refused input whenever a headset was
  // present, which left no way to override it.
  const handleSliderChange = (value: number[]) => {
    const next = value[0];
    setChatmixValue(next);
    setManual(true);
    debouncedManualMutate(next);
  };

  const followWheel = () => {
    setManual(false);
    chatmixSetManualMutation(null).catch(() => {});
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

        <div className="flex items-center justify-between gap-3 pt-1">
          <p className="text-[11px] text-muted-foreground">
            {manual
              ? 'Set by hand. The headset wheel is ignored until you follow it again.'
              : hasHeadset
                ? 'Following the headset wheel.'
                : 'No headset connected. Drag to set the balance by hand.'}
          </p>
          {manual && hasHeadset && (
            <Button variant="ghost" size="sm" className="text-[11px]" onClick={followWheel}>
              Follow wheel
            </Button>
          )}
        </div>
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
