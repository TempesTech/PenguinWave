import { AppNavigation } from '@/components/app-navigation';
import { DndBoard } from '@/components/dnd-board';
import { PageHeader } from '@/components/page-header';

export default function AudioOrganizer() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <AppNavigation />
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 animate-in fade-in duration-300">
        <PageHeader
          title="Routing"
          subtitle="Drag applications into sinks to control where they play."
        />
        <DndBoard />
      </div>
    </div>
  );
}
