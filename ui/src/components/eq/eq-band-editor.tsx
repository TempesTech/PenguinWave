import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Trash2 } from 'lucide-react';
import {
  EqBand,
  FILTER_TYPE_LABELS,
  FilterType,
  FREQ_MAX,
  FREQ_MIN,
  GAIN_MAX,
  GAIN_MIN,
  Q_MAX,
  Q_MIN,
} from '@/types/eq';

interface EqBandEditorProps {
  band: EqBand;
  index: number;
  onChange: (band: EqBand) => void;
  onRemove: () => void;
  canRemove: boolean;
}

/** Numeric/keyboard editing for the selected band (a11y complement to the curve). */
export function EqBandEditor({ band, index, onChange, onRemove, canRemove }: EqBandEditorProps) {
  const numberField = (
    label: string,
    unit: string,
    value: number,
    min: number,
    max: number,
    step: number,
    apply: (value: number) => void,
  ) => (
    <div className="flex flex-col gap-1.5">
      <Label className="text-[11px] text-muted-foreground">
        {label} <span className="text-muted-foreground/60">{unit}</span>
      </Label>
      <Input
        type="number"
        value={value}
        min={min}
        max={max}
        step={step}
        className="h-8 font-mono text-xs"
        onChange={(event) => {
          const parsed = parseFloat(event.target.value);
          if (!Number.isNaN(parsed)) apply(Math.min(max, Math.max(min, parsed)));
        }}
        data-testid={`eq-band-editor-${label.toLowerCase()}`}
      />
    </div>
  );

  return (
    <div
      className="grid grid-cols-2 items-end gap-3 sm:grid-cols-[auto_minmax(8rem,1fr)_repeat(3,minmax(5rem,1fr))_auto_auto]"
      data-testid="eq-band-editor"
    >
      <div className="flex h-8 items-center self-end">
        <span className="flex h-6 w-6 items-center justify-center rounded-md bg-primary/15 font-mono text-xs font-semibold text-primary">
          {index + 1}
        </span>
      </div>

      <div className="flex flex-col gap-1.5">
        <Label className="text-[11px] text-muted-foreground">Type</Label>
        <Select
          value={band.filter_type}
          onValueChange={(value) => onChange({ ...band, filter_type: value as FilterType })}
        >
          <SelectTrigger className="h-8 text-xs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {Object.entries(FILTER_TYPE_LABELS).map(([value, label]) => (
              <SelectItem key={value} value={value}>
                {label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {numberField('Freq', 'Hz', Math.round(band.freq), FREQ_MIN, FREQ_MAX, 1, (freq) =>
        onChange({ ...band, freq }),
      )}
      {numberField('Gain', 'dB', band.gain_db, GAIN_MIN, GAIN_MAX, 0.5, (gain_db) =>
        onChange({ ...band, gain_db }),
      )}
      {numberField('Q', '', Number(band.q.toFixed(2)), Q_MIN, Q_MAX, 0.1, (q) =>
        onChange({ ...band, q }),
      )}

      <div className="flex h-8 items-center gap-2 self-end">
        <Switch
          checked={band.enabled}
          onCheckedChange={(enabled) => onChange({ ...band, enabled })}
          data-testid="eq-band-editor-enabled"
        />
      </div>

      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8 self-end text-muted-foreground hover:text-destructive"
        onClick={onRemove}
        disabled={!canRemove}
        title="Remove band"
        data-testid="eq-band-editor-remove"
      >
        <Trash2 className="h-4 w-4" />
      </Button>
    </div>
  );
}
