import { AppNavigation } from '@/components/app-navigation';
import { RunningApplications } from '@/components/running-applications';
import { HeadsetPanel } from '@/components/headset-panel';
import { DevicePicker } from '@/components/device-picker';
import { PageHeader } from '@/components/page-header';

export default function Dashboard() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <AppNavigation />
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 animate-in fade-in duration-300">
        <PageHeader
          title="Dashboard"
          subtitle="Live audio routing and ChatMix at a glance."
          action={<DevicePicker />}
        />
        <div className="space-y-8 max-w-5xl">
          <HeadsetPanel />
          <RunningApplications />
        </div>
      </div>
    </div>
  );
}
