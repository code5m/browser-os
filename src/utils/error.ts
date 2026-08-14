import { useLayoutStore } from "../stores/useLayoutStore";

// 统一包裹 bridge 调用：成功可选 toast，失败统一 toast 错误，且不抛异常中断 UI
export async function withToast<T>(p: Promise<T>, ok?: string): Promise<T | undefined> {
  try {
    const r = await p;
    if (ok) useLayoutStore().showToast(ok);
    return r;
  } catch (e: any) {
    useLayoutStore().showToast(e?.message ?? "操作失败");
    return undefined;
  }
}
