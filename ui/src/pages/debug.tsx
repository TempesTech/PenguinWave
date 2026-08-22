import { AppNavigation } from '@/components/app-navigation';
import { HidDevices } from '@/components/hid-devices';
import { PageHeader } from '@/components/page-header';

export default function Debug() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <AppNavigation />
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 animate-in fade-in duration-300">
        <PageHeader
          title="System"
          subtitle="Hardware enumeration and debug utilities."
        />
        <div className="max-w-2xl">
          <HidDevices />
        </div>
      </div>
    </div>
  );
}
