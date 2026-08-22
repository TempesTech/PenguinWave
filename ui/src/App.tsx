import { Switch, Route } from "wouter";
import { queryClient } from "./lib/queryClient";
import { QueryClientProvider } from "@tanstack/react-query";
import { Toaster } from "@/components/ui/toaster";
import { TooltipProvider } from "@/components/ui/tooltip";
import { ThemeProvider } from "@/components/theme-provider";
import Dashboard from "@/pages/dashboard";
import AudioOrganizer from "@/pages/audio-organizer";
import SinkManager from "@/pages/sink-manager";
import Equalizer from "@/pages/equalizer";
import Debug from "@/pages/debug";
import Maintenance from "@/pages/maintenance";
import NotFound from "@/pages/not-found";
import { DaemonStatus } from "@/components/daemon-status";

function Router() {
  return (
    <Switch>
      <Route path="/" component={Dashboard} />
      <Route path="/dnd" component={AudioOrganizer} />
      <Route path="/sink-manager" component={SinkManager} />
      <Route path="/equalizer" component={Equalizer} />
      <Route path="/debug" component={Debug} />
      <Route path="/maintenance" component={Maintenance} />
      <Route component={NotFound} />
    </Switch>
  );
}

function App() {
  return (
    <ThemeProvider defaultTheme="dark" storageKey="penguin-wave-theme">
      <QueryClientProvider client={queryClient}>
        <TooltipProvider>
          <Toaster />
          <DaemonStatus>
            <Router />
          </DaemonStatus>
        </TooltipProvider>
      </QueryClientProvider>
    </ThemeProvider>
  );
}

export default App;
