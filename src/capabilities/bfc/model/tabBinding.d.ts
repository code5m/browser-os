export interface BfcTabIdentity {id: string; url: string; title?: string}
export function isBilibiliPage(raw: string): boolean;
export function selectBfcReadbackTab<T extends BfcTabIdentity>(tabs: readonly T[], selectedId: string, requestedId?: string): T;
export function safeBfcTabSummary(tab: BfcTabIdentity): {tabId: string; title: string; url: string; isFavorites: boolean};
