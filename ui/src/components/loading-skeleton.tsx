import { Skeleton } from '@/components/ui/skeleton';

type Variant = 'rows' | 'cards' | 'board';

interface LoadingSkeletonProps {
  variant?: Variant;
  count?: number;
  className?: string;
}

export function LoadingSkeleton({
  variant = 'rows',
  count = 3,
  className,
}: LoadingSkeletonProps) {
  if (variant === 'cards') {
    return (
      <div className={'space-y-3 ' + (className ?? '')}>
        {Array.from({ length: count }).map((_, i) => (
          <Skeleton key={i} className="h-24 w-full" />
        ))}
      </div>
    );
  }

  if (variant === 'board') {
    return (
      <div
        className={
          'grid grid-cols-1 md:grid-cols-3 gap-4 ' + (className ?? '')
        }
      >
        {Array.from({ length: count }).map((_, i) => (
          <Skeleton key={i} className="h-48 w-full" />
        ))}
      </div>
    );
  }

  return (
    <div className={'space-y-2 ' + (className ?? '')}>
      {Array.from({ length: count }).map((_, i) => (
        <Skeleton key={i} className="h-10 w-full" />
      ))}
    </div>
  );
}
