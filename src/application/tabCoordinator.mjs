/**
 * Route logical tab intents to their existing owners. The projection remains
 * read-only; this coordinator owns no tab records or native resources.
 */
export function createTabCoordinator({ resources, activate, close, enabled = () => true }) {
  let tail = Promise.resolve();
  const find = (key) => resources().find((tab) => tab.key === key);
  const run = (operation) => {
    const result = tail.then(() => enabled() ? operation() : false);
    tail = result.catch(() => {});
    return result;
  };
  async function activateOne(key) {
    const target = find(key);
    if (!target) return false;
    if (target.active) return true;
    return (await activate(target)) !== false;
  }
  async function closeOne(key) {
    const target = find(key);
    if (!target) return true; // A completed earlier close is already satisfied.
    const before = resources();
    const index = before.findIndex(tab => tab.key === key);
    const neighbor = target.active ? (before[index + 1] || before[index - 1]) : null;
    if ((await close(target)) === false) return false;
    if (neighbor && find(neighbor.key)) return activateOne(neighbor.key);
    return true;
  }
  return {
    activate: (key) => run(() => activateOne(key)),
    close: (key) => run(() => closeOne(key)),
    closeActive: () => run(async () => {
      const active = resources().find((tab) => tab.active);
      return active ? closeOne(active.key) : false;
    }),
    cycle: (delta) => run(async () => {
      const tabs = resources();
      if (!tabs.length) return false;
      const direction = delta < 0 ? -1 : 1;
      const index = tabs.findIndex((tab) => tab.active);
      const next = index < 0 ? (direction > 0 ? 0 : tabs.length - 1)
        : (index + direction + tabs.length) % tabs.length;
      return activateOne(tabs[next].key);
    }),
    closeOthers: (key) => run(async () => {
      // Check the retained tab and its navigation guard before closing anything.
      if (!await activateOne(key)) return false;
      const keys = resources().filter((tab) => tab.key !== key).map((tab) => tab.key);
      for (const other of keys) {
        // Do not continue a destructive batch if its retained target disappears.
        if (!enabled() || !find(key)) return false;
        if (!await closeOne(other)) return false;
      }
      return true;
    }),
  };
}
