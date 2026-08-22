import { AudioDevice, AudioPort, PipeWireNode } from '@/types/audio_manager';
import { Query } from '@/queries/query.ts';
import { request, send } from '@/lib/daemon';
import { outputDevice, sinkNameById, sinkToNode } from '@/lib/adapters';

//#region Queries
const QUERY_KEYS = {
  GET_CUSTOM_VIRTUAL_SINKS: 'getCustomVirtualSinks',
  GET_DEFAULT_SINK: 'getDefaultSink',
  GET_NODES: 'getNodes',
  GET_NODE_PORTS: 'getNodePorts',
  GET_OUTPUT_DEVICES: 'getOutputDevices',
  GET_SINK_ROUTE: 'getSinkRoute',
} as const;

export const GET_CUSTOM_VIRTUAL_SINKS_QUERY: Query<PipeWireNode[]> = {
  queryKey: [QUERY_KEYS.GET_CUSTOM_VIRTUAL_SINKS],
  queryFn: async () =>
    (await request({ method: 'sink.list_custom' }, 'sinks')).map(sinkToNode),
};

export const GET_DEFAULT_SINK_QUERY: Query<string> = {
  queryKey: [QUERY_KEYS.GET_DEFAULT_SINK],
  queryFn: async () => request({ method: 'sink.default' }, 'sink_name'),
};

export const GET_NODES_QUERY: Query<PipeWireNode[]> = {
  queryKey: [QUERY_KEYS.GET_NODES],
  queryFn: async () => (await request({ method: 'sink.list' }, 'sinks')).map(sinkToNode),
};

/// Ports are addressed by node name: `pw-link` works on names, and the same
/// name can cover several nodes, so an id alone would not identify one.
export function nodePortsQuery(nodeId: number | null): Query<AudioPort[]> {
  return {
    queryKey: [QUERY_KEYS.GET_NODE_PORTS, nodeId?.toString() ?? 'none'],
    queryFn: async () => {
      if (nodeId === null) return [];
      const ports = await request(
        { method: 'graph.list_node_ports', params: { node: await sinkNameById(nodeId) } },
        'ports',
      );
      return ports.map((p) => ({
        name: p.port_name,
        description: `${p.node_name}:${p.port_name}`,
        direction: p.direction,
      }));
    },
  };
}

export const NODE_PORTS_QUERY_KEY_PREFIX = QUERY_KEYS.GET_NODE_PORTS;

export const GET_OUTPUT_DEVICES_QUERY: Query<AudioDevice[]> = {
  queryKey: [QUERY_KEYS.GET_OUTPUT_DEVICES],
  queryFn: async () =>
    (await request({ method: 'sink.list_output_devices' }, 'output_devices')).map(
      outputDevice,
    ),
};

export function sinkRouteQuery(sinkName: string): Query<string | null> {
  return {
    queryKey: [QUERY_KEYS.GET_SINK_ROUTE, sinkName],
    queryFn: async () => {
      const route = await request(
        { method: 'sink.current_route', params: { sink: sinkName } },
        'route',
      );
      return route?.device ?? null;
    },
  };
}
//#endregion

//#region Mutations
export async function createVirtualSinkMutation(
  name: string,
  description: string,
): Promise<void> {
  await send({
    method: 'sink.create',
    params: { config: { name, display_name: description } },
  });
}

/// Sinks are deleted by name now; the daemon owns module ids.
export async function deleteVirtualSinkMutation(sinkId: number): Promise<void> {
  await send({ method: 'sink.delete', params: { name: await sinkNameById(sinkId) } });
}

function portRef(spec: string) {
  const [node, port] = spec.split(':');
  return { node_name: node ?? spec, port_name: port ?? '', id: null };
}

export async function linkPortsMutation(
  sourcePort: string,
  targetPort: string,
): Promise<void> {
  await send({
    method: 'graph.link',
    params: { source: portRef(sourcePort), target: portRef(targetPort) },
  });
}

export async function unlinkPortsMutation(
  sourcePort: string,
  targetPort: string,
): Promise<void> {
  await send({
    method: 'graph.unlink',
    params: { source: portRef(sourcePort), target: portRef(targetPort) },
  });
}

export async function setSinkVolumeMutation(
  sinkId: number,
  volume: number,
): Promise<void> {
  await send({
    method: 'sink.set_volume',
    params: { sink: await sinkNameById(sinkId), pct: Math.round(volume) },
  });
}

export async function routeSinkToDeviceMutation(
  sinkName: string,
  targetDevice: string,
): Promise<void> {
  await send({
    method: 'sink.route_to_device',
    params: { sink: sinkName, device: targetDevice },
  });
}
//#endregion
