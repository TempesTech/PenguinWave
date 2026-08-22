import { AlertTriangle } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';

interface EqSafeModeBannerProps {
  onReset: () => void;
}

export function EqSafeModeBanner({ onReset }: EqSafeModeBannerProps) {
  return (
    <Alert variant="destructive" data-testid="eq-safe-mode-banner">
      <AlertTriangle className="h-4 w-4" />
      <AlertTitle>Equalizer disabled (safe mode)</AlertTitle>
      <AlertDescription className="flex items-center justify-between gap-4">
        <span>
          The EQ audio process crashed repeatedly and was disabled to keep your audio
          working. Sound now flows directly to your output device, without EQ.
        </span>
        <Button variant="outline" size="sm" onClick={onReset} data-testid="eq-safe-mode-reset">
          Re-enable EQ
        </Button>
      </AlertDescription>
    </Alert>
  );
}
