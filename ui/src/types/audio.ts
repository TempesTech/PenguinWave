export interface Application {
  id: string;
  name: string;
  description: string;
  icon: string;
  icon_path?: string;
  category: 'game' | 'chat' | 'media' | 'others';
}

export interface ApplicationStream {
  id: number;
  name: string;
  icon: string;
  assigned_sink_id: number;
  icon_path: string;
  is_muted: boolean;
  volume: number;
}

export interface AudioPort {
  name: string;
  description: string;
  direction: string; // "input" or "output"
}

export interface AudioCategory {
  id: string;
  name: string;
  description?: string;
  volume: number;
  applications: Application[];
  ports?: AudioPort[];
}