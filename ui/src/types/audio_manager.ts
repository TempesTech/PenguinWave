export interface PipeWireNode {
  id: number;
  name: string;
  description: string;
  node_type: string;
  module_id: number;
  volume: number;
}

export interface AudioPort {
  name: string;
  description: string;
  direction: string;
}

export interface AudioDevice {
  id: number;
  name: string;
  description: string;
  ports: AudioPort[];
}