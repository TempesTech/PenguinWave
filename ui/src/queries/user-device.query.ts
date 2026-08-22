import { Query } from './query';
import { UserDevice } from '@/types/user-device';
import { request, send } from '@/lib/daemon';

const QUERY_KEYS = {
  GET_USER_DEVICES: 'getUserDevices',
} as const;

export const GET_USER_DEVICES_QUERY: Query<UserDevice[]> = {
  queryKey: [QUERY_KEYS.GET_USER_DEVICES],
  queryFn: async () => request({ method: 'device.list_user' }, 'user_devices'),
};

export async function addUserDeviceMutation(device: UserDevice): Promise<void> {
  await send({ method: 'device.add_user', params: { device } });
}

export async function removeUserDeviceMutation(name: string): Promise<void> {
  await send({ method: 'device.remove_user', params: { name } });
}
