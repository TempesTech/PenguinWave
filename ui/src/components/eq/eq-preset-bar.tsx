import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Save, Trash2 } from 'lucide-react';
import { EqPresetMeta } from '@/types/eq';

interface EqPresetBarProps {
  presets: EqPresetMeta[];
  onApply: (name: string) => void;
  onSave: (name: string) => void;
  onDelete: (name: string) => void;
}

export function EqPresetBar({ presets, onApply, onSave, onDelete }: EqPresetBarProps) {
  const [selected, setSelected] = useState<string>('');
  const [saveName, setSaveName] = useState('');

  const selectedPreset = presets.find((p) => p.name === selected);

  return (
    <div className="flex flex-wrap items-center gap-2">
      <Select
        value={selected}
        onValueChange={(name) => {
          setSelected(name);
          onApply(name);
        }}
      >
        <SelectTrigger className="h-8 w-44" data-testid="eq-preset-select">
          <SelectValue placeholder="Preset…" />
        </SelectTrigger>
        <SelectContent>
          {presets.map((preset) => (
            <SelectItem key={preset.name} value={preset.name}>
              {preset.name}
              {preset.builtin ? ' •' : ''}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      {selectedPreset && !selectedPreset.builtin && (
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 text-destructive"
          title="Delete preset"
          onClick={() => {
            onDelete(selected);
            setSelected('');
          }}
          data-testid="eq-preset-delete"
        >
          <Trash2 className="h-4 w-4" />
        </Button>
      )}

      <div className="flex items-center gap-1">
        <Input
          placeholder="Save as…"
          value={saveName}
          onChange={(event) => setSaveName(event.target.value)}
          className="h-8 w-36"
          data-testid="eq-preset-name"
        />
        <Button
          variant="secondary"
          size="sm"
          className="h-8"
          disabled={!saveName.trim()}
          onClick={() => {
            onSave(saveName.trim());
            setSaveName('');
          }}
          data-testid="eq-preset-save"
        >
          <Save className="mr-1 h-3.5 w-3.5" />
          Save
        </Button>
      </div>
    </div>
  );
}
