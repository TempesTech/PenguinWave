import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Plus, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useToast } from '@/hooks/use-toast';
import {
  GET_USER_DEVICES_QUERY,
  addUserDeviceMutation,
  removeUserDeviceMutation,
} from '@/queries/user-device.query';
import { GET_OUTPUT_DEVICES_QUERY } from '@/queries/sink_manager.query';
import { UserDevice } from '@/types/user-device';

function AddDeviceModal({ onAdded }: { onAdded: () => void }) {
  const { toast } = useToast();
  const [open, setOpen] = useState(false);
  const [name, setName] = useState('');
  const [vendorId, setVendorId] = useState('');
  const [productId, setProductId] = useState('');
  const [pipewireSink, setPipewireSink] = useState('');

  const { data: outputDevices = [] } = useQuery({
    ...GET_OUTPUT_DEVICES_QUERY,
    enabled: open,
  });

  const { mutate, isPending } = useMutation({
    mutationFn: (device: UserDevice) => addUserDeviceMutation(device),
    onSuccess() {
      setOpen(false);
      setName('');
      setVendorId('');
      setProductId('');
      setPipewireSink('');
      onAdded();
      toast({ title: 'Device added' });
    },
    onError(e) {
      toast({ title: 'Failed to add device', description: e.message, variant: 'destructive' });
    },
  });

  const handleSubmit = () => {
    if (!name.trim()) return;
    mutate({
      name: name.trim(),
      vendor_id: vendorId.trim() || null,
      product_id: productId.trim() || null,
      pipewire_sink: pipewireSink || null,
    });
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2">
          <Plus className="h-4 w-4" />
          Add Device
        </Button>
      </DialogTrigger>
      <DialogContent className="bg-background border-border">
        <DialogHeader>
          <DialogTitle>Add Custom Device</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 pt-2">
          <div className="space-y-1.5">
            <Label htmlFor="ud-name">Name *</Label>
            <Input
              id="ud-name"
              placeholder="Sennheiser Momentum 4"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="bg-secondary"
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label htmlFor="ud-vid">Vendor ID (hex)</Label>
              <Input
                id="ud-vid"
                placeholder="1395"
                value={vendorId}
                onChange={(e) => setVendorId(e.target.value)}
                className="bg-secondary font-mono"
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="ud-pid">Product ID (hex)</Label>
              <Input
                id="ud-pid"
                placeholder="0030"
                value={productId}
                onChange={(e) => setProductId(e.target.value)}
                className="bg-secondary font-mono"
              />
            </div>
          </div>
          <p className="text-[11px] text-muted-foreground -mt-1">
            Vendor/Product IDs are optional — required only for udev rule generation.
          </p>
          <div className="space-y-1.5">
            <Label>PipeWire Sink (output routing)</Label>
            <Select value={pipewireSink} onValueChange={setPipewireSink}>
              <SelectTrigger className="bg-secondary">
                <SelectValue placeholder="Select output sink…" />
              </SelectTrigger>
              <SelectContent className="bg-background border-border">
                {outputDevices.map((d) => (
                  <SelectItem key={d.name} value={d.name} className="text-foreground hover:bg-accent">
                    <span className="font-mono text-xs">{d.name}</span>
                    {d.description && (
                      <span className="ml-2 text-muted-foreground">{d.description}</span>
                    )}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <Button
            className="w-full"
            onClick={handleSubmit}
            disabled={!name.trim() || isPending}
          >
            Add Device
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export function UserDevices() {
  const queryClient = useQueryClient();
  const { toast } = useToast();

  const { data: devices = [], isPending } = useQuery(GET_USER_DEVICES_QUERY);

  const { mutate: remove } = useMutation({
    mutationFn: removeUserDeviceMutation,
    onSuccess() {
      queryClient.invalidateQueries({ queryKey: GET_USER_DEVICES_QUERY.queryKey });
      toast({ title: 'Device removed' });
    },
    onError(e) {
      toast({ title: 'Failed to remove device', description: e.message, variant: 'destructive' });
    },
  });

  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: GET_USER_DEVICES_QUERY.queryKey });

  return (
    <section className="rounded-xl border border-border bg-card p-6 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-semibold text-foreground">Custom Devices</h3>
          <p className="text-[11px] text-muted-foreground mt-0.5">
            Non-HID headsets for output routing and udev rule generation.
          </p>
        </div>
        <AddDeviceModal onAdded={invalidate} />
      </div>

      {isPending && (
        <p className="text-sm text-muted-foreground">Loading…</p>
      )}

      {!isPending && devices.length === 0 && (
        <p className="text-sm text-muted-foreground">No custom devices added yet.</p>
      )}

      {devices.length > 0 && (
        <ul className="space-y-2">
          {devices.map((d) => (
            <li
              key={d.name}
              className="flex items-center justify-between rounded-lg border border-border bg-secondary/40 px-4 py-3"
            >
              <div className="min-w-0">
                <p className="text-sm font-medium text-foreground truncate">{d.name}</p>
                <p className="text-[11px] font-mono text-muted-foreground">
                  {d.vendor_id && d.product_id
                    ? `VID ${d.vendor_id} · PID ${d.product_id}`
                    : 'No HID IDs'}
                  {d.pipewire_sink && (
                    <span className="ml-2 text-primary/70">→ {d.pipewire_sink}</span>
                  )}
                </p>
              </div>
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8 text-muted-foreground hover:text-destructive shrink-0"
                onClick={() => remove(d.name)}
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
