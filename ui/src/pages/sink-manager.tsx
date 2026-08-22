import { useEffect, useState } from 'react';
import { ChevronDown } from 'lucide-react';
import { AppNavigation } from '@/components/app-navigation';
import { AudioDevices } from '@/components/audio-device';
import { CreateSinkModal } from '@/components/create-sink-modal';
import { PageHeader } from '@/components/page-header';
import { PortLinkPanel } from '@/components/port-link-panel';
import { Button } from '@/components/ui/button';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import { useToast } from '@/hooks/use-toast';
import { useMutation, useQuery } from '@tanstack/react-query';
import {
  createVirtualSinkMutation,
  deleteVirtualSinkMutation,
  GET_CUSTOM_VIRTUAL_SINKS_QUERY,
} from '@/queries/sink_manager.query';
import { PipeWireNode } from '@/types/audio_manager';
import { queryClient } from '@/lib/queryClient.ts';

export default function SinkManager() {
  const { toast } = useToast();
  const [portLinkOpen, setPortLinkOpen] = useState(false);

  const {
    data: customSinks,
    error: customSinksError,
    isError: customSinksHasError,
    isPending,
  } = useQuery<PipeWireNode[]>(GET_CUSTOM_VIRTUAL_SINKS_QUERY);

  const { mutate: deleteSink } = useMutation({
    mutationFn: deleteVirtualSinkMutation,
    async onSuccess(_, moduleId) {
      await invalidateCustomSinksQuery();
      toast({
        title: 'Sink deleted',
        description: `Module ${moduleId} unloaded.`,
      });
    },
    onError(error) {
      toast({
        title: 'Failed to delete sink',
        description: error.message,
        variant: 'destructive',
      });
    },
  });

  const { mutate: createSink } = useMutation({
    mutationFn: ({ name, description }: { name: string; description: string }) =>
      createVirtualSinkMutation(name, description),
    async onSuccess(moduleId) {
      await invalidateCustomSinksQuery();
      toast({
        title: 'Sink created',
        description: `New sink loaded as module ${moduleId}.`,
      });
    },
    onError(error) {
      toast({
        title: 'Failed to create sink',
        description: error.message,
        variant: 'destructive',
      });
    },
  });

  const handleCreateSink = (name: string, description: string) => {
    createSink({ name, description });
  };

  const invalidateCustomSinksQuery = async () => {
    await queryClient.invalidateQueries({
      queryKey: [...GET_CUSTOM_VIRTUAL_SINKS_QUERY.queryKey],
    });
  };

  useEffect(() => {
    if (customSinksHasError && customSinksError) {
      toast({
        title: 'Failed to retrieve custom sinks',
        description: customSinksError.message,
        variant: 'destructive',
      });
    }
  }, [customSinksHasError, customSinksError]);

  return (
    <div className="min-h-screen bg-background text-foreground">
      <AppNavigation />
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 animate-in fade-in duration-300">
        <PageHeader
          title="Sinks"
          subtitle="Create virtual sinks, control their volume, link ports."
          action={<CreateSinkModal onCreateSink={handleCreateSink} />}
        />

        <div className="space-y-8 max-w-5xl">
          <AudioDevices
            customSinks={customSinks}
            isLoading={isPending}
            onRefresh={invalidateCustomSinksQuery}
            onDeleteSink={deleteSink}
          />

          <Collapsible open={portLinkOpen} onOpenChange={setPortLinkOpen}>
            <CollapsibleTrigger asChild>
              <Button
                variant="ghost"
                className="w-full justify-between px-3 -mx-3 text-[11px] uppercase tracking-[0.18em] text-muted-foreground hover:text-foreground hover:bg-accent/50"
              >
                <span>Port Linking · advanced</span>
                <ChevronDown
                  className={
                    'h-4 w-4 transition-transform ' +
                    (portLinkOpen ? 'rotate-180' : '')
                  }
                />
              </Button>
            </CollapsibleTrigger>
            <CollapsibleContent className="pt-3">
              <PortLinkPanel />
            </CollapsibleContent>
          </Collapsible>
        </div>
      </div>
    </div>
  );
}
