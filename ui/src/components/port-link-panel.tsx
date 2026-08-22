import { useMemo, useState } from 'react';
import { Link2, Unlink2 } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useMutation, useQuery } from '@tanstack/react-query';
import { PipeWireNode } from '@/types/audio_manager';
import { toast } from '@/hooks/use-toast';
import {
  GET_NODES_QUERY,
  linkPortsMutation,
  nodePortsQuery,
  unlinkPortsMutation,
} from '@/queries/sink_manager.query';

function formatPort(nodeName: string, portName: string): string {
  return `${nodeName}:${portName}`;
}

interface NodePortSelectProps {
  label: string;
  direction: 'output' | 'input';
  nodes: PipeWireNode[] | undefined;
  selectedNodeId: number | null;
  onSelectNode: (nodeId: number | null) => void;
  selectedPort: string | null;
  onSelectPort: (portName: string | null) => void;
}

function NodePortSelect({
  label,
  direction,
  nodes,
  selectedNodeId,
  onSelectNode,
  selectedPort,
  onSelectPort,
}: NodePortSelectProps) {
  const portsQuery = useQuery(nodePortsQuery(selectedNodeId));
  const filteredPorts = useMemo(
    () => (portsQuery.data ?? []).filter((p) => p.direction === direction),
    [portsQuery.data, direction],
  );

  return (
    <div className="space-y-2">
      <div className="text-xs uppercase tracking-wide text-muted-foreground">
        {label}
      </div>
      <Select
        value={selectedNodeId !== null ? selectedNodeId.toString() : undefined}
        onValueChange={(value) => {
          onSelectNode(Number(value));
          onSelectPort(null);
        }}
      >
        <SelectTrigger className="bg-background border-border text-foreground">
          <SelectValue placeholder="Select node" />
        </SelectTrigger>
        <SelectContent className="bg-background border-border">
          {(nodes ?? []).map((n) => (
            <SelectItem
              key={n.id}
              value={n.id.toString()}
              className="text-foreground hover:bg-accent"
            >
              {n.name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      <Select
        value={selectedPort ?? undefined}
        onValueChange={(value) => onSelectPort(value)}
        disabled={selectedNodeId === null || filteredPorts.length === 0}
      >
        <SelectTrigger className="bg-background border-border text-foreground">
          <SelectValue
            placeholder={
              selectedNodeId === null
                ? 'Pick node first'
                : portsQuery.isPending
                ? 'Loading ports…'
                : filteredPorts.length === 0
                ? `No ${direction} ports`
                : 'Select port'
            }
          />
        </SelectTrigger>
        <SelectContent className="bg-background border-border">
          {filteredPorts.map((p) => (
            <SelectItem
              key={p.name}
              value={p.name}
              className="text-foreground hover:bg-accent"
            >
              {p.name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}

export function PortLinkPanel() {
  const { data: nodes } = useQuery(GET_NODES_QUERY);

  const [sourceNodeId, setSourceNodeId] = useState<number | null>(null);
  const [sourcePort, setSourcePort] = useState<string | null>(null);
  const [targetNodeId, setTargetNodeId] = useState<number | null>(null);
  const [targetPort, setTargetPort] = useState<string | null>(null);

  const sourceNode = nodes?.find((n) => n.id === sourceNodeId);
  const targetNode = nodes?.find((n) => n.id === targetNodeId);

  const canAct =
    sourceNode != null &&
    sourcePort != null &&
    targetNode != null &&
    targetPort != null;

  const linkMutation = useMutation({
    mutationFn: ({ src, tgt }: { src: string; tgt: string }) =>
      linkPortsMutation(src, tgt),
    onSuccess() {
      toast({ title: 'Linked', description: `${src()} → ${tgt()}` });
    },
    onError(error) {
      toast({
        title: 'Link failed',
        description: error.message,
        variant: 'destructive',
      });
    },
  });

  const unlinkMutation = useMutation({
    mutationFn: ({ src, tgt }: { src: string; tgt: string }) =>
      unlinkPortsMutation(src, tgt),
    onSuccess() {
      toast({ title: 'Unlinked', description: `${src()} → ${tgt()}` });
    },
    onError(error) {
      toast({
        title: 'Unlink failed',
        description: error.message,
        variant: 'destructive',
      });
    },
  });

  function src(): string {
    return sourceNode && sourcePort ? formatPort(sourceNode.name, sourcePort) : '';
  }
  function tgt(): string {
    return targetNode && targetPort ? formatPort(targetNode.name, targetPort) : '';
  }

  const handleLink = () => {
    if (!canAct) return;
    linkMutation.mutate({ src: src(), tgt: tgt() });
  };

  const handleUnlink = () => {
    if (!canAct) return;
    unlinkMutation.mutate({ src: src(), tgt: tgt() });
  };

  return (
    <Card className="bg-card text-card-foreground border-border">
      <CardHeader>
        <CardTitle className="text-foreground">Port Linking</CardTitle>
        <div className="text-xs text-muted-foreground">
          Wire a source output port to a target input port (pw-link).
        </div>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <NodePortSelect
            label="Source (output)"
            direction="output"
            nodes={nodes}
            selectedNodeId={sourceNodeId}
            onSelectNode={setSourceNodeId}
            selectedPort={sourcePort}
            onSelectPort={setSourcePort}
          />
          <NodePortSelect
            label="Target (input)"
            direction="input"
            nodes={nodes}
            selectedNodeId={targetNodeId}
            onSelectNode={setTargetNodeId}
            selectedPort={targetPort}
            onSelectPort={setTargetPort}
          />
        </div>

        <div className="mt-4 flex items-center gap-2">
          <Button
            onClick={handleLink}
            disabled={!canAct || linkMutation.isPending}
            className="gap-2"
          >
            <Link2 className="h-4 w-4" />
            Link
          </Button>
          <Button
            onClick={handleUnlink}
            variant="outline"
            disabled={!canAct || unlinkMutation.isPending}
            className="gap-2"
          >
            <Unlink2 className="h-4 w-4" />
            Unlink
          </Button>
          {canAct && (
            <span className="ml-auto text-xs text-muted-foreground font-mono truncate">
              {src()} → {tgt()}
            </span>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
