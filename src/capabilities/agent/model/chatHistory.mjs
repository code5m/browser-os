// Pure project-free chat transport helper. Rust enforces the same byte limits.
const encoder = new TextEncoder()
export function utf8Bytes(text) { return encoder.encode(text).byteLength }
export function truncateUtf8(text, limit) {
  let length = 0
  let result = ''
  for (const char of text) {
    const width = utf8Bytes(char)
    if (length + width > limit) break
    result += char
    length += width
  }
  return result
}
export function selectChatHistory(lines, maxTurns = 12, maxTotalBytes = 16000) {
  let available = maxTotalBytes
  const selected = []
  for (const line of lines.slice(-maxTurns).reverse()) {
    if (!['user', 'assistant'].includes(line.role)) continue
    const content = truncateUtf8(line.content, Math.min(3000, available))
    if (!content.trim()) continue
    available -= utf8Bytes(content)
    selected.unshift({ role: line.role, content })
    if (available <= 0) break
  }
  return selected
}
// Some older WebKitGTK builds expose crypto.getRandomValues but not randomUUID.
export function createChatRequestId(cryptoSource = globalThis.crypto) {
  if (cryptoSource?.randomUUID) return cryptoSource.randomUUID()
  if (!cryptoSource?.getRandomValues) throw Error('CHAT_SECURE_RANDOM_UNAVAILABLE')
  const bytes = new Uint8Array(16)
  cryptoSource.getRandomValues(bytes)
  bytes[6] = (bytes[6] & 0x0f) | 0x40
  bytes[8] = (bytes[8] & 0x3f) | 0x80
  const hex = Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('')
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`
}
