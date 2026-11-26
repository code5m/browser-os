// UI layout settlement across Browser and Grid. Neither capability grants
// native access to the other. Only the existing native control plane can
// acknowledge a hide/position; this gate prevents stale UI dispatch ordering.
import { ref } from "vue";

export const nativeSurfaceBlocked = ref(false);
export const nativeSurfaceSettling = ref(false);
export const nativeSurfaceIssue = ref("");
const issued = new Set<Promise<unknown>>();

export function trackNativeLayout(work: Promise<unknown>): void {
  issued.add(work);
  const release = () => { issued.delete(work); };
  void work.then(release, release);
}
export async function settleNativeLayouts(): Promise<void> {
  // Callers synchronously close the gate before taking this snapshot.
  // A timeout never releases ownership of an unsettled native request.
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    await Promise.race([
      Promise.allSettled([...issued]),
      new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(new Error("NATIVE_LAYOUT_SETTLEMENT_PENDING")), 15000);
      }),
    ]);
  } finally { if (timer !== undefined) clearTimeout(timer); }
}
