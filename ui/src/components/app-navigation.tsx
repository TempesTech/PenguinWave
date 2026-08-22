import { Link, useLocation } from 'wouter';
import { Waves } from 'lucide-react';
import { ThemeSwitcher } from './theme-switcher';
import { cn } from '@/lib/utils';

const NAV_ITEMS = [
  { href: '/', label: 'Dashboard' },
  { href: '/dnd', label: 'Routing' },
  { href: '/sink-manager', label: 'Sinks' },
  { href: '/equalizer', label: 'Equalizer' },
  { href: '/debug', label: 'System' },
  { href: '/maintenance', label: 'Maintenance' },
];

export function AppNavigation() {
  const [location] = useLocation();

  return (
    <nav className="bg-background/80 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-b border-border sticky top-0 z-50">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex justify-between items-center h-14">
          <div className="flex items-center gap-2">
            <Waves className="h-5 w-5 text-primary" />
            <span className="text-sm font-semibold tracking-tight text-foreground">
              PenguinWave
            </span>
          </div>

          <div className="hidden md:flex items-center gap-1">
            {NAV_ITEMS.map((item) => {
              const active = location === item.href;
              return (
                <Link key={item.href} href={item.href}>
                  <button
                    className={cn(
                      'px-3 py-1.5 rounded-md text-xs font-medium uppercase tracking-wider transition-colors',
                      active
                        ? 'text-foreground bg-accent'
                        : 'text-muted-foreground hover:text-foreground hover:bg-accent/50',
                    )}
                  >
                    {item.label}
                  </button>
                </Link>
              );
            })}
          </div>

          <div className="flex items-center gap-1">
            <ThemeSwitcher />
          </div>
        </div>
      </div>
    </nav>
  );
}
