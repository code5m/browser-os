/** UI identities only; native Owners remain the filesystem authorization authority. */
export function documentPathKey(root: string, path: string): string;
export function sameDocumentPath(a: string, b: string): boolean;
export function containsDocumentPath(base: string, path: string): boolean;
export function documentPathName(path: string): string;
export function documentPathParent(path: string): string;
export function joinDocumentPath(base: string, relative: string): string;
export function relativeDocumentPath(base: string, path: string): string | null;
export function replaceDocumentPathPrefix(path: string, from: string, to: string): string;
