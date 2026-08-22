export type Capability =
  | 'CapSidetone'
  | 'CapBatteryStatus'
  | 'CapNotificationSound'
  | 'CapLights'
  | 'CapInactiveTime'
  | 'CapChatMixStatus'
  | 'CapVoicePrompts'
  | 'CapRotateToMute'
  | 'CapEqualizerPreset'
  | 'CapEqualizer'
  | 'CapParametricEqualizer'
  | 'CapMicrophoneMuteLedBrightness'
  | 'CapMicrophoneVolume'
  | 'CapVolumeLimiter'
  | 'CapBtWhenPoweredOn'
  | 'CapBtCallVolume'
  | 'NumCapabilities';

export interface HeadsetDescriptor {
  vendor_id: number;
  product_id: number;
  name: string;
  is_connected: boolean;
  capabilities: Capability[];
}
