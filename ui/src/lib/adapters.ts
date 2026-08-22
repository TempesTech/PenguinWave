// Proto wire types -> the view types the components already use.
//
// The adapter lives here so a protocol change stops at the query layer instead
// of rippling through every component.

import type { SinkInfo } from '@proto/SinkInfo';
import type { StreamInfo } from '@proto/StreamInfo';
import type { StreamRef } from '@proto/StreamRef';
import type { DeviceDescriptor } from '@proto/DeviceDescriptor';
import type { OutputDevice } from '@proto/OutputDevice';
import { request } from '@/lib/daemon';
import type { ApplicationStream } from '@/types/audio';
import type { AudioDevice, PipeWireNode } from '@/types/audio_manager';
import type { Capability, HeadsetDescriptor } from '@/types/headset';

/// Icons never change for a key, so one fetch per key per session is enough.
const iconCache = new Map<string, string>();

export async function iconFor(key: string | null): Promise<string> {
  if (!key) return '';
  const cached = iconCache.get(key);
  if (cached !== undefined) return cached;

  const icon = (await request({ method: 'stream.icon', params: { key } }, 'icon')) ?? '';
  const dataUrl = icon ? `data:image/png;base64,${icon}` : '';
  iconCache.set(key, dataUrl);
  return dataUrl;
}

/// Sinks are addressed by name on the wire but by numeric id throughout the
/// UI, so the mapping is resolved here rather than in every component.
export async function sinkNameById(id: number): Promise<string> {
  const sinks = await request({ method: 'sink.list' }, 'sinks');
  const sink = sinks.find((s) => s.id === id);
  if (!sink) throw new Error(`no sink with id ${id}`);
  return sink.name;
}

export function sinkToNode(sink: SinkInfo): PipeWireNode {
  return {
    id: sink.id,
    name: sink.name,
    description: sink.description,
    node_type: sink.managed ? 'virtual' : 'device',
    module_id: 0,
    volume: sink.volume,
  };
}

/// Streams keyed by application name, the shape the board renders from.
export async function streamsByApp(
  streams: StreamInfo[],
  sinks: SinkInfo[],
): Promise<Record<string, ApplicationStream[]>> {
  const sinkId = new Map(sinks.map((s) => [s.name, s.id]));
  const icons = await Promise.all(streams.map((s) => iconFor(s.icon_key)));

  const grouped: Record<string, ApplicationStream[]> = {};
  streams.forEach((stream, i) => {
    const entry: ApplicationStream = {
      id: stream.index,
      name: stream.name,
      icon: icons[i],
      icon_path: '',
      assigned_sink_id: sinkId.get(stream.sink) ?? -1,
      is_muted: stream.is_muted,
      volume: stream.volume,
    };
    (grouped[stream.name] ??= []).push(entry);
  });
  return grouped;
}

/// Address a stream by index alone.
///
/// The daemon revalidates the reference before mutating, so it does not need
/// the name and pid the enumeration carried.
export function streamRef(index: number): StreamRef {
  return { index, app_name: '', pid: null };
}

export function outputDevice(device: OutputDevice): AudioDevice {
  return {
    id: device.id,
    name: device.name,
    description: device.description,
    ports: [],
  };
}

/// Capability names differ: the wire uses snake_case, the UI the old `Cap`
/// prefixed spelling.
const CAPABILITY_NAMES: Record<string, Capability> = {
  sidetone: 'CapSidetone',
  battery_status: 'CapBatteryStatus',
  notification_sound: 'CapNotificationSound',
  lights: 'CapLights',
  inactive_time: 'CapInactiveTime',
  chat_mix_status: 'CapChatMixStatus',
  voice_prompts: 'CapVoicePrompts',
  rotate_to_mute: 'CapRotateToMute',
  equalizer_preset: 'CapEqualizerPreset',
  equalizer: 'CapEqualizer',
  parametric_equalizer: 'CapParametricEqualizer',
  microphone_mute_led_brightness: 'CapMicrophoneMuteLedBrightness',
  microphone_volume: 'CapMicrophoneVolume',
  volume_limiter: 'CapVolumeLimiter',
  bt_when_powered_on: 'CapBtWhenPoweredOn',
  bt_call_volume: 'CapBtCallVolume',
};

export function headset(device: DeviceDescriptor): HeadsetDescriptor {
  return {
    vendor_id: device.vendor_id,
    product_id: device.product_id,
    name: device.name,
    is_connected: device.presence === 'connected',
    capabilities: device.capabilities
      .map((c) => CAPABILITY_NAMES[c])
      .filter((c): c is Capability => c !== undefined),
  };
}
