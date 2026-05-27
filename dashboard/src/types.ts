export interface RuntimeEvent {
  event: string;
  plugin_id: string;
  crash_count?: number;
  memory_bytes?: number;
}

export interface PluginInfo {
  id: string;
  runtime_type: string;
  status: string;
  memory_bytes: number;
  crash_count: number;
}

export interface SnapshotResponse { total: number; plugins: PluginInfo[] }
