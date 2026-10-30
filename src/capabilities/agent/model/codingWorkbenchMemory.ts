// Process-local only: never persist source text or credentials to browser storage.
import { ref } from 'vue';

// A remounted view must not start a second MCP operation while the first is alive.
export const manualMcpRunning = ref(false);
let snapshot: Record<string, unknown> | null = null;

export function saveWorkbenchMemory(value: Record<string, unknown>) {
  snapshot = value;
}

export function takeWorkbenchMemory(): Record<string, any> | null {
  const value = snapshot;
  snapshot = null;
  return value;
}
