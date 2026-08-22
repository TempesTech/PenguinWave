import { useState } from 'react';
import { GripVertical, Inbox, Layers, Volume2 } from 'lucide-react';
import {
  DndContext,
  DragEndEvent,
  DragOverEvent,
  DragOverlay,
  DragStartEvent,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  useDroppable,
  useDraggable,
} from '@dnd-kit/core';
import { Slider } from '@/components/ui/slider';
import { useAudioState } from '@/hooks/use-audio-state';
import { cn } from '@/lib/utils';
import { Application, AudioCategory } from '@/types/audio';
import { EmptyState } from '@/components/empty-state';
import { LoadingSkeleton } from '@/components/loading-skeleton';

const UNASSIGNED_ID = 'others';

function AppRow({
  app,
  isDragging,
  muted,
}: {
  app: Application;
  isDragging?: boolean;
  muted?: boolean;
}) {
  return (
    <div
      className={cn(
        'group flex items-center gap-2 px-2 py-1.5 rounded-md text-sm',
        muted ? 'bg-background/40' : 'bg-background/60',
        'hover:bg-accent transition-colors',
        isDragging && 'opacity-40',
      )}
    >
      <div className="h-6 w-6 rounded bg-muted flex items-center justify-center overflow-hidden shrink-0">
        {app.icon_path ? (
          <img
            src={app.icon_path}
            alt=""
            className="w-full h-full object-contain"
          />
        ) : (
          <span className="text-[10px] opacity-60">🎵</span>
        )}
      </div>
      <span
        className={cn(
          'truncate flex-1',
          muted ? 'text-muted-foreground' : 'text-foreground',
        )}
      >
        {app.name}
      </span>
      <GripVertical className="h-3.5 w-3.5 text-muted-foreground opacity-0 group-hover:opacity-60 transition-opacity" />
    </div>
  );
}

function DraggableAppRow({
  app,
  isDragging,
  muted,
}: {
  app: Application;
  isDragging: boolean;
  muted?: boolean;
}) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    isDragging: beingDragged,
  } = useDraggable({ id: app.id });
  const style = transform
    ? { transform: `translate3d(${transform.x}px, ${transform.y}px, 0)` }
    : undefined;
  return (
    <div
      ref={setNodeRef}
      style={style}
      {...listeners}
      {...attributes}
      className={cn(
        'touch-none cursor-grab active:cursor-grabbing',
        beingDragged && 'opacity-30',
      )}
    >
      <AppRow app={app} isDragging={isDragging} muted={muted} />
    </div>
  );
}

interface ColumnProps {
  category: AudioCategory;
  isUnassigned: boolean;
  isOver: boolean;
  activeId: string | null;
  updateCategoryVolume: (id: string, volume: number) => void;
}

function Column({
  category,
  isUnassigned,
  isOver,
  activeId,
  updateCategoryVolume,
}: ColumnProps) {
  const { setNodeRef, isOver: droppableOver } = useDroppable({ id: category.id });
  const isActiveTarget = isOver || droppableOver;
  const count = category.applications.length;

  return (
    <div
      ref={setNodeRef}
      className={cn(
        'rounded-xl p-3 space-y-3 transition-colors',
        isUnassigned
          ? 'border border-dashed border-border bg-muted/15'
          : 'border border-border bg-card',
        isActiveTarget && 'border-primary bg-primary/5',
      )}
    >
      <header className="flex items-center justify-between gap-2">
        <div className="flex items-baseline gap-2 min-w-0">
          {isUnassigned && (
            <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
              Source
            </span>
          )}
          <h3
            className={cn(
              'text-sm font-medium truncate',
              isUnassigned ? 'text-muted-foreground' : 'text-foreground',
            )}
          >
            {category.name}
          </h3>
          <span className="text-[11px] font-mono tabular-nums text-muted-foreground shrink-0">
            {count}
          </span>
        </div>
        {!isUnassigned && (
          <div className="flex items-center gap-2 w-32 shrink-0">
            <Volume2 className="h-3 w-3 text-muted-foreground" />
            <Slider
              value={[category.volume]}
              onValueChange={(v) => updateCategoryVolume(category.id, v[0])}
              max={100}
              min={0}
              step={1}
              className="flex-1"
            />
            <span className="text-[10px] font-mono tabular-nums text-muted-foreground w-7 text-right">
              {category.volume}%
            </span>
          </div>
        )}
      </header>

      {count === 0 ? (
        <div className="text-[11px] text-muted-foreground text-center py-6 border border-dashed border-border/60 rounded-md">
          {isUnassigned ? 'No unassigned streams' : 'Drop apps here'}
        </div>
      ) : (
        <div className="space-y-1">
          {category.applications.map((app) => (
            <DraggableAppRow
              key={app.id}
              app={app}
              isDragging={activeId === app.id}
              muted={isUnassigned}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export function DndBoard() {
  const {
    categoriesNew: categories,
    categoriesPending,
    updateCategoryVolume,
    moveApplication,
  } = useAudioState();
  const [activeId, setActiveId] = useState<string | null>(null);
  const [overId, setOverId] = useState<string | null>(null);
  const [draggedApp, setDraggedApp] = useState<Application | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } }),
  );

  const handleDragStart = (event: DragStartEvent) => {
    const { active } = event;
    setActiveId(active.id as string);
    const foundApp = categories
      ?.flatMap((cat) => cat.applications)
      .find((app) => app.id === active.id);
    setDraggedApp(foundApp ?? null);
  };

  const handleDragOver = (event: DragOverEvent) => {
    setOverId(event.over ? (event.over.id as string) : null);
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (over && active.id !== over.id && categories) {
      const sourceCategory = categories.find((cat) =>
        cat.applications.some((app) => app.id === active.id),
      );
      const targetCategory = categories.find((cat) => cat.id === over.id);
      if (sourceCategory && targetCategory && sourceCategory.id !== targetCategory.id) {
        moveApplication(active.id as string, sourceCategory.id, targetCategory.id);
      }
    }
    setActiveId(null);
    setOverId(null);
    setDraggedApp(null);
  };

  if (categoriesPending) {
    return <LoadingSkeleton variant="board" count={3} />;
  }

  if (!categories || categories.length === 0) {
    return (
      <EmptyState
        icon={Layers}
        title="No categories yet"
        description="Create a virtual sink in the Sinks page to start routing apps."
      />
    );
  }

  const unassigned = categories.find((c) => c.id === UNASSIGNED_ID);
  const sinks = categories.filter((c) => c.id !== UNASSIGNED_ID);
  const sinksAreEmpty = sinks.length === 0;

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragStart={handleDragStart}
      onDragOver={handleDragOver}
      onDragEnd={handleDragEnd}
    >
      <div className="space-y-6">
        {unassigned && (
          <Column
            category={unassigned}
            isUnassigned
            isOver={overId === unassigned.id}
            activeId={activeId}
            updateCategoryVolume={updateCategoryVolume}
          />
        )}

        {sinksAreEmpty ? (
          <div className="rounded-xl border border-dashed border-border">
            <EmptyState
              icon={Inbox}
              title="No virtual sinks"
              description="Create one in the Sinks page to drop streams into."
            />
          </div>
        ) : (
          <div className="space-y-2">
            <div className="text-[10px] uppercase tracking-[0.18em] text-muted-foreground px-1">
              Sinks
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
              {sinks.map((category) => (
                <Column
                  key={category.id}
                  category={category}
                  isUnassigned={false}
                  isOver={overId === category.id}
                  activeId={activeId}
                  updateCategoryVolume={updateCategoryVolume}
                />
              ))}
            </div>
          </div>
        )}
      </div>

      <DragOverlay dropAnimation={null}>
        {activeId && draggedApp ? (
          <div className="w-72 shadow-lg shadow-primary/10 ring-1 ring-primary/30 rounded-md">
            <AppRow app={draggedApp} isDragging />
          </div>
        ) : null}
      </DragOverlay>
    </DndContext>
  );
}
