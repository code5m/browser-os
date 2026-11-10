export type NativeTabLoadFailureKind = 'tls' | 'platform-limited' | 'load';
export interface NativeTabLoadFailureRecord {kind: NativeTabLoadFailureKind; at: number}
export const NATIVE_LOAD_FAILURE_TTL_MS: number;
export function classifyNativeTabLoadFailure(status: string, message: string): NativeTabLoadFailureKind | null;
export function recentNativeTabLoadFailure(record: NativeTabLoadFailureRecord | undefined, now?: number): NativeTabLoadFailureKind | null;
