import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useSelectedDevice } from '@/hooks/use-selected-device';

function deviceKey(vendorId: number, productId: number): string {
  return `${vendorId}:${productId}`;
}

export function DevicePicker() {
  const { supportedDevices, selectedDevice, isPending, selectDevice } = useSelectedDevice();

  if (isPending) {
    return (
      <div className="text-sm text-muted-foreground">Detecting devices…</div>
    );
  }

  if (!supportedDevices || supportedDevices.length === 0) {
    return (
      <div className="text-sm text-muted-foreground">
        No supported headsets found.
      </div>
    );
  }

  const currentValue = selectedDevice
    ? deviceKey(selectedDevice.vendor_id, selectedDevice.product_id)
    : undefined;

  return (
    <Select
      value={currentValue}
      onValueChange={(value) => {
        const [v, p] = value.split(':').map(Number);
        selectDevice(v, p);
      }}
    >
      <SelectTrigger className="w-[260px] bg-background border-border text-foreground">
        <SelectValue placeholder="Select headset" />
      </SelectTrigger>
      <SelectContent className="bg-background border-border">
        {supportedDevices.map((d) => (
          <SelectItem
            key={deviceKey(d.vendor_id, d.product_id)}
            value={deviceKey(d.vendor_id, d.product_id)}
            className="text-foreground hover:bg-accent"
          >
            {d.name}
            {d.is_connected ? '' : ' (disconnected)'}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
