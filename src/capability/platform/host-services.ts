export type HostServiceId = 'bridge' | 'layout' | 'workbench' | 'graph-layout' | 'redact-secrets' | 'browser-context'

export interface BrowserContextPort {
  readonly activeTabId: string
  readonly activeUrl: string
  readonly recentlyClosed: readonly { url: string; title?: string }[]
  readonly aiNavOpen: boolean
  openTab(url: string): Promise<unknown>
  captureTextPreview(tabId: string): Promise<string>
  adoptRestoredTab(tab: unknown): void
  activateGrid(): Promise<void>
  setAiNavOpen(value: boolean): void
}

export interface HostServiceRegistry {
  register<T>(id: HostServiceId, service: T): void
  get<T>(id: HostServiceId): T | undefined
  require<T>(id: HostServiceId): T
  count(): number
}

export function createHostServiceRegistry(): HostServiceRegistry {
  const services = new Map<HostServiceId, unknown>()
  return {
    register(id, service) { services.set(id, service) },
    get(id) { return services.get(id) as any },
    require(id) {
      const service = services.get(id)
      if (!service) throw new Error(`Host service 未注册: ${id}`)
      return service as any
    },
    count() { return services.size },
  }
}

export const hostServices = createHostServiceRegistry()
