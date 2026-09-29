export type CapabilityEventType =
  | 'capability.discovered'
  | 'capability.registered'
  | 'capability.activating'
  | 'capability.activated'
  | 'capability.failed'
  | 'capability.deactivating'
  | 'capability.deactivated'

export interface CapabilityEvent {
  type: CapabilityEventType
  capabilityId: string
  error?: string
  at: number
}

export interface CapabilityEventBus {
  emit(event: CapabilityEvent): void
  subscribe(listener: (event: CapabilityEvent) => void): () => void
  history(): CapabilityEvent[]
}

export function createCapabilityEventBus(): CapabilityEventBus {
  const listeners = new Set<(event: CapabilityEvent) => void>()
  const events: CapabilityEvent[] = []
  return {
    emit(event) { events.push(event); listeners.forEach((listener) => listener(event)) },
    subscribe(listener) { listeners.add(listener); return () => listeners.delete(listener) },
    history() { return [...events] },
  }
}
