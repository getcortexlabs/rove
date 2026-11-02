import { pathToFileURL } from 'node:url'

const endpoint = 'https://getcortexlabs.com/api/rove/releases'

export async function notifyBeta({ releaseId, secret }, {
  request = fetch,
  pause = ms => new Promise(resolve => setTimeout(resolve, ms)),
  log = console.log,
} = {}) {
  if (!Number.isSafeInteger(releaseId) || releaseId <= 0) throw new Error('A published release ID is required.')
  if (typeof secret !== 'string' || secret.length < 32) throw new Error('Configure ROVE_RELEASE_SECRET before publishing a release.')
  let failures = 0
  for (let attempt = 0; attempt < 1200; attempt++) {
    let response
    try {
      response = await request(endpoint, {
        method: 'POST',
        headers: { authorization: `Bearer ${secret}`, 'content-type': 'application/json' },
        body: JSON.stringify({ releaseId }),
        signal: AbortSignal.timeout(25_000),
      })
    } catch {
      // A timeout may follow an accepted email; the server retries with the same key.
    }
    if (!response || response.status === 429 || response.status >= 500) {
      if (++failures >= 8) throw new Error('Release notifications could not finish. Check the service and rerun this workflow with the same release ID.')
      await pause(Math.min(60_000, 2000 * 2 ** failures))
      continue
    }
    if (!response.ok) {
      throw new Error(`Release notification service returned ${response.status}. Check release readiness, credentials and deliveries requiring review.`)
    }
    const result = await response.json()
    if (result.releaseId !== releaseId || typeof result.complete !== 'boolean'
        || ['sent', 'pending', 'skipped', 'review'].some(key => !Number.isSafeInteger(result[key]) || result[key] < 0)) {
      throw new Error('Invalid notification receipt.')
    }
    log(`Release ${releaseId}: ${result.sent} accepted, ${result.pending} pending, ${result.skipped} skipped, ${result.review} need review.`)
    if (result.complete && result.pending === 0 && result.review === 0) return result
    if (result.review) throw new Error('Some deliveries require review before another attempt.')
    failures = 0
    // One message per request, below the provider's default two requests/second.
    await pause(750)
  }
  throw new Error('Notification time budget reached. Rerun with the same release ID to continue.')
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    await notifyBeta({ releaseId: Number(process.env.ROVE_RELEASE_ID), secret: process.env.ROVE_RELEASE_SECRET })
  } catch (error) {
    console.error(error.message)
    process.exitCode = 1
  }
}
