import { useState } from 'react';
import { Plus } from 'lucide-react';
import { AppNavigation } from '@/components/app-navigation';
import { PageHeader } from '@/components/page-header';
import { EqBandEditor } from '@/components/eq/eq-band-editor';
import { EqCurve, formatFreq } from '@/components/eq/eq-curve';
import { EqPresetBar } from '@/components/eq/eq-preset-bar';
import { EqSafeModeBanner } from '@/components/eq/eq-safe-mode-banner';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Skeleton } from '@/components/ui/skeleton';
import { Slider } from '@/components/ui/slider';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';
import { useEq } from '@/hooks/use-eq';
import {
  CHAIN_LABELS,
  defaultBand,
  EqChainId,
  GAIN_MAX,
  GAIN_MIN,
  MAX_BANDS,
} from '@/types/eq';

export default function Equalizer() {
  const {
    state,
    isPending,
    presets,
    setBand,
    setPreamp,
    setChainEnabled,
    addBand,
    removeBand,
    applyPreset,
    savePreset,
    deletePreset,
    resetSafeMode,
  } = useEq();

  const [activeChain, setActiveChain] = useState<EqChainId>('game');
  const [selectedBand, setSelectedBand] = useState<number | null>(null);

  const chain = state?.chains[activeChain];
  const disabled = state?.safe_mode ?? false;

  return (
    <div className="min-h-screen bg-background">
      <AppNavigation />
      <main className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
        <PageHeader
          title="Equalizer"
          subtitle="Parametric EQ on the Game and Chat outputs"
          action={
            <EqPresetBar
              presets={presets}
              onApply={(name) => applyPreset(activeChain, name)}
              onSave={(name) => savePreset(activeChain, name)}
              onDelete={deletePreset}
            />
          }
        />

        {state?.safe_mode && (
          <div className="mb-4">
            <EqSafeModeBanner onReset={resetSafeMode} />
          </div>
        )}

        {isPending || !chain ? (
          <div className="space-y-4">
            <Skeleton className="h-9 w-64" />
            <Skeleton className="aspect-[8/3] w-full rounded-xl" />
            <Skeleton className="h-16 w-full" />
          </div>
        ) : (
          <div className="space-y-4">
            {/* chain selector + enable */}
            <div className="flex items-center justify-between gap-3">
              <Tabs
                value={activeChain}
                onValueChange={(value) => {
                  setActiveChain(value as EqChainId);
                  setSelectedBand(null);
                }}
              >
                <TabsList>
                  {(Object.keys(CHAIN_LABELS) as EqChainId[]).map((id) => (
                    <TabsTrigger key={id} value={id} data-testid={`eq-chain-tab-${id}`}>
                      {CHAIN_LABELS[id]}
                    </TabsTrigger>
                  ))}
                </TabsList>
              </Tabs>

              <div className="flex items-center gap-2">
                <Label
                  htmlFor="eq-enabled"
                  className={cn(
                    'text-xs',
                    chain.enabled ? 'text-foreground' : 'text-muted-foreground',
                  )}
                >
                  {chain.enabled ? 'Active' : 'Bypassed'}
                </Label>
                <Switch
                  id="eq-enabled"
                  checked={chain.enabled}
                  disabled={disabled}
                  onCheckedChange={(enabled) => setChainEnabled(activeChain, enabled)}
                  data-testid="eq-chain-enabled"
                />
              </div>
            </div>

            {/* curve */}
            <div className="rounded-xl border border-border bg-card p-2">
              <EqCurve
                chain={chain}
                selectedBand={selectedBand}
                onSelectBand={setSelectedBand}
                onBandChange={(index, band) => setBand(activeChain, index, band)}
              />
            </div>

            {/* band strip + preamp */}
            <div className="flex flex-wrap items-center gap-2">
              <div className="flex flex-wrap items-center gap-1.5" data-testid="eq-band-strip">
                {chain.bands.map((band, index) => (
                  <button
                    key={index}
                    onClick={() => setSelectedBand(selectedBand === index ? null : index)}
                    className={cn(
                      'rounded-md border px-2 py-1 font-mono text-[11px] transition-colors',
                      selectedBand === index
                        ? 'border-primary bg-primary/15 text-primary'
                        : 'border-border text-muted-foreground hover:border-primary/40 hover:text-foreground',
                      !band.enabled && 'opacity-50 line-through',
                    )}
                    data-testid={`eq-band-chip-${index}`}
                  >
                    {formatFreq(band.freq)}
                  </button>
                ))}
                <Button
                  variant="outline"
                  size="sm"
                  className="h-7 px-2 text-xs"
                  disabled={disabled || chain.bands.length >= MAX_BANDS}
                  onClick={() => {
                    addBand(activeChain, defaultBand());
                    setSelectedBand(chain.bands.length);
                  }}
                  data-testid="eq-add-band"
                >
                  <Plus className="mr-1 h-3 w-3" />
                  Band
                </Button>
              </div>

              <div className="ml-auto flex min-w-56 items-center gap-3">
                <Label className="whitespace-nowrap font-mono text-[11px] text-muted-foreground">
                  Preamp {chain.preamp_db > 0 ? '+' : ''}
                  {chain.preamp_db.toFixed(1)} dB
                </Label>
                <Slider
                  value={[chain.preamp_db]}
                  min={GAIN_MIN}
                  max={GAIN_MAX}
                  step={0.5}
                  className="w-40"
                  disabled={disabled}
                  onValueChange={([value]) => setPreamp(activeChain, value)}
                  data-testid="eq-preamp-slider"
                />
              </div>
            </div>

            {/* editor panel: fixed slot so selecting a band doesn't shift the page */}
            <div className="min-h-[76px] rounded-xl border border-border bg-card p-3">
              {selectedBand !== null && chain.bands[selectedBand] ? (
                <EqBandEditor
                  band={chain.bands[selectedBand]}
                  index={selectedBand}
                  canRemove={chain.bands.length > 1}
                  onChange={(band) => setBand(activeChain, selectedBand, band)}
                  onRemove={() => {
                    removeBand(activeChain, selectedBand);
                    setSelectedBand(null);
                  }}
                />
              ) : (
                <p className="flex h-full min-h-[52px] items-center text-xs text-muted-foreground">
                  Select a band on the curve to edit it. Drag to change frequency and gain,
                  scroll to adjust Q, arrow keys for fine control.
                </p>
              )}
            </div>
          </div>
        )}
      </main>
    </div>
  );
}
