export interface TabResource {
  key: string;
  id: string;
  kind: string;
  active: boolean;
  view?: string;
  path?: string | null;
}
export interface TabCoordinator {
  activate(key: string): Promise<boolean>;
  close(key: string): Promise<boolean>;
  closeOthers(key: string): Promise<boolean>;
  closeActive(): Promise<boolean>;
  cycle(delta: number): Promise<boolean>;
}
export function createTabCoordinator(owners: {
  resources(): TabResource[];
  activate(tab: TabResource): boolean | void | Promise<boolean | void>;
  close(tab: TabResource): boolean | void | Promise<boolean | void>;
  enabled?(): boolean;
}): TabCoordinator;
