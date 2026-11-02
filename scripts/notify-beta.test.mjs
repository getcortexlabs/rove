import { test } from 'node:test'
import assert from 'node:assert/strict'
import { notifyBeta } from './notify-beta.mjs'

const options = { releaseId: 123, secret: 'test-secret'.repeat(4) }
const receipt = (pending = 0) => Response.json({ releaseId: 123, sent: 1, pending, skipped: 0, review: 0, complete: pending === 0 })
const quiet = { pause: async () => {}, log: () => {} }

test('drains a queue without transmitting emails or accepting caller-supplied URLs', async () => {
  let calls = 0
  const result = await notifyBeta(options, { ...quiet, request: async (url, config) => {
    assert.equal(url, 'https://getcortexlabs.com/api/rove/releases')
    assert.deepEqual(JSON.parse(config.body), { releaseId: 123 })
    return receipt(calls++ === 0 ? 1 : 0)
  } })
  assert.equal(result.complete, true)
  assert.equal(calls, 2)
})

test('retries transient failures with the same release and bounds failed attempts', async () => {
  let calls = 0
  await notifyBeta(options, { ...quiet, request: async () => calls++ < 2 ? new Response('', { status: 503 }) : receipt() })
  assert.equal(calls, 3)
  calls = 0
  await assert.rejects(notifyBeta(options, { ...quiet, request: async () => { calls++; throw new Error('timeout') } }), /could not finish/)
  assert.equal(calls, 8)
})

test('fails on missing setup, operator-review responses and malformed receipts', async () => {
  await assert.rejects(notifyBeta({ ...options, secret: undefined }, quiet), /Configure/)
  await assert.rejects(notifyBeta({ ...options, releaseId: NaN }, quiet), /release ID/)
  await assert.rejects(notifyBeta(options, { ...quiet, request: async () => new Response('', { status: 409 }) }), /409/)
  await assert.rejects(notifyBeta(options, { ...quiet, request: async () => Response.json({ complete: true }) }), /Invalid/)
})
