import { useEffect } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  GET_SELECTED_DEVICE_QUERY,
  GET_SUPPORTED_DEVICES_QUERY,
  setSelectedDeviceMutation,
} from '@/queries/device.query';
import { toast } from '@/hooks/use-toast';

const STORAGE_KEY = 'penguin-wave-selected-device';

interface StoredSelection {
  vendor_id: number;
  product_id: number;
}

function readStored(): StoredSelection | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (
      typeof parsed?.vendor_id === 'number' &&
      typeof parsed?.product_id === 'number'
    ) {
      return parsed as StoredSelection;
    }
    return null;
  } catch {
    return null;
  }
}

function writeStored(selection: StoredSelection): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(selection));
}

export function useSelectedDevice() {
  const queryClient = useQueryClient();

  const { data: supportedDevices, isPending: supportedPending } = useQuery(
    GET_SUPPORTED_DEVICES_QUERY,
  );
  const { data: selectedDevice, isPending: selectedPending } = useQuery(
    GET_SELECTED_DEVICE_QUERY,
  );

  const { mutate: selectDeviceMutate } = useMutation({
    mutationFn: ({ vendorId, productId }: { vendorId: number; productId: number }) =>
      setSelectedDeviceMutation(vendorId, productId),
    onSuccess(_data, { vendorId, productId }) {
      writeStored({ vendor_id: vendorId, product_id: productId });
      queryClient.invalidateQueries({ queryKey: GET_SELECTED_DEVICE_QUERY.queryKey });
    },
    onError(error) {
      toast({
        title: 'Failed to select device',
        description: error.message,
        variant: 'destructive',
      });
    },
  });

  useEffect(() => {
    if (selectedDevice) return;
    if (!supportedDevices || supportedDevices.length === 0) return;

    const stored = readStored();
    if (stored) {
      const match = supportedDevices.find(
        (d) => d.vendor_id === stored.vendor_id && d.product_id === stored.product_id,
      );
      if (match?.is_connected) {
        selectDeviceMutate({ vendorId: match.vendor_id, productId: match.product_id });
        return;
      }
    }

    const connected = supportedDevices.filter((d) => d.is_connected);
    if (connected.length === 1) {
      const only = connected[0];
      selectDeviceMutate({ vendorId: only.vendor_id, productId: only.product_id });
    }
  }, [supportedDevices, selectedDevice, selectDeviceMutate]);

  return {
    supportedDevices,
    selectedDevice,
    isPending: supportedPending || selectedPending,
    selectDevice: (vendorId: number, productId: number) =>
      selectDeviceMutate({ vendorId, productId }),
  };
}
