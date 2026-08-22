const KNOWN: Record<string, string> = {
  game_sink: 'Game',
  chat_sink: 'Chat',
};

/**
 * Pick a friendly display label for a sink.
 * 1. Hard-coded mapping for well-known sinks (game_sink/chat_sink).
 * 2. Sink description if meaningfully different from name.
 * 3. Raw sink name as fallback.
 */
export function sinkLabel(name: string, description?: string): string {
  if (name in KNOWN) return KNOWN[name];
  if (description && description.trim() && description !== name) {
    return description;
  }
  return name;
}
