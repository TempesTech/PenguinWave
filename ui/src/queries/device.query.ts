import { Query } from './query';
import { HeadsetDescriptor } from '@/types/headset';
import { ChatMix } from '@proto/ChatMix';
import { request, send } from '@/lib/daemon';
import { headset } from '@/lib/adapters';

const QUERY_KEYS = {
  GET_SUPPORTED_DEVICES: 'getSupportedDevices',
  GET_SELECTED_DEVICE: 'getSelectedDevice',
  GET_CHATMIX: 'getChatMix',
} as const;

export const GET_SUPPORTED_DEVICES_QUERY: Query<HeadsetDescriptor[]> = {
  queryKey: [QUERY_KEYS.GET_SUPPORTED_DEVICES],
  queryFn: async () =>
    (await request({ method: 'device.list_supported' }, 'devices')).map(headset),
};

export const GET_SELECTED_DEVICE_QUERY: Query<HeadsetDescriptor | null> = {
  queryKey: [QUERY_KEYS.GET_SELECTED_DEVICE],
  queryFn: async () => {
    const [selected, devices] = await Promise.all([
      request({ method: 'device.get_selected' }, 'selected_device'),
      request({ method: 'device.list_supported' }, 'devices'),
    ]);
    if (!selected) return null;
    const found = devices.find(
      (d) => d.vendor_id === selected.vendor_id && d.product_id === selected.product_id,
    );
    return found ? headset(found) : null;
  },
};

export const GET_CHATMIX_QUERY: Query<ChatMix> = {
  queryKey: [QUERY_KEYS.GET_CHATMIX],
  queryFn: async () => (await request({ method: 'session.snapshot' }, 'snapshot')).chatmix,
};

export async function setSelectedDeviceMutation(
  vendorId: number,
  productId: number,
): Promise<void> {
  await send({
    method: 'device.set_selected',
    params: { device: { vendor_id: vendorId, product_id: productId } },
  });
}

/// `null` hands the balance back to the headset wheel.
export async function chatmixSetManualMutation(value: number | null): Promise<void> {
  await send({
    method: 'chatmix.set_manual',
    params: { value: value === null ? null : Math.round(value) },
  });
}
