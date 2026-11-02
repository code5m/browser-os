import assert from 'node:assert/strict'
import { selectChatHistory, truncateUtf8, utf8Bytes, createChatRequestId } from '../src/capabilities/agent/model/chatHistory.mjs'
const source = '简体中文测试🙂'.repeat(1500)
assert.equal(utf8Bytes('中文'), 6)
assert.ok(utf8Bytes(truncateUtf8(source, 3000)) <= 3000)
assert.ok(utf8Bytes(truncateUtf8(source, 3001)) <= 3001)
assert.ok(!truncateUtf8('a🙂b', 4).includes('🙂'))
const history = selectChatHistory(Array.from({ length: 20 }, (_, i) => ({ role: i % 2 ? 'assistant' : 'user', content: source })))
assert.ok(history.length <= 12)
assert.ok(history.every(turn => utf8Bytes(turn.content) <= 3000))
assert.ok(history.reduce((sum, turn) => sum + utf8Bytes(turn.content), 0) <= 16000)
assert.ok(history.every(turn => !turn.content.includes('\uFFFD')))
assert.equal(selectChatHistory([{ role: 'system', content: 'inject' }]).length, 0)
assert.equal(selectChatHistory([{ role: 'user', content: '中文' }])[0].content, '中文')

const uuid = createChatRequestId({ getRandomValues: bytes => { for(let i=0; i<bytes.length; i++) bytes[i]=i; return bytes } })
assert.match(uuid, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/)
console.log('AGENT_CHAT_HISTORY_UTF8=PASS')
