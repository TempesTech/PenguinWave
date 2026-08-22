import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Separator } from '@/components/ui/separator';
import { useMutation } from '@tanstack/react-query';
import { request } from '@/lib/daemon';
import { FormEvent, useState } from 'react';

/// `init_app` is gone: the daemon initialises itself and enumerates
/// continuously, so this re-reads what it currently sees.
async function listDevices(): Promise<string> {
  const devices = await request({ method: 'device.list_supported' }, 'devices');
  if (devices.length === 0) return 'No supported devices found.';
  return devices
    .map(
      (d) =>
        `${d.vendor_id.toString(16).padStart(4, '0')}:` +
        `${d.product_id.toString(16).padStart(4, '0')}  ${d.name}  ${d.presence}`,
    )
    .join('\n');
}

export function HidDevices() {
  const [deviceList, setDeviceList] = useState('');

  const { mutate: refreshDevices } = useMutation({
    mutationFn: listDevices,
    onSuccess: (result) => {
      setDeviceList(result);
    },
    onError: (error: Error) => {
      setDeviceList(error.message);
    },
  });

  const handleFindDevice = (e: FormEvent) => {
    e.preventDefault();
    refreshDevices();
  };

  return (
    <Card className='bg-card text-card-foreground border-border'>
      <CardHeader>
        <CardTitle className='text-foreground'>HID Devices</CardTitle>
      </CardHeader>
      <CardContent className='space-y-4'>
        <Button onClick={handleFindDevice} className='w-full' variant='secondary'>
          Find Device
        </Button>

        <Separator />

        <div className='space-y-2'>
          <Label className='text-foreground'>Detected devices</Label>
          <Textarea readOnly value={deviceList} rows={8} className='font-mono text-xs' />
        </div>
      </CardContent>
    </Card>
  );
}
