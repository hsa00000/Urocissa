import { getHashToken } from '@/db/db'

self.addEventListener('install', () => {
  console.log('[Service Worker] Installing...')
  const result = self as unknown as ServiceWorkerGlobalScope
  result.skipWaiting().catch((err: unknown) => {
    console.error('[Service Worker] skipWaiting() failed:', err)
  })
})

self.addEventListener('activate', (event: unknown) => {
  if (!(event instanceof ExtendableEvent)) {
    return
  }

  const result = self as unknown as ServiceWorkerGlobalScope
  console.log('[Service Worker] Activating...')

  event.waitUntil(
    (async () => {
      try {
        await result.clients.claim()
        console.log('[Service Worker] Clients claimed.')
      } catch (err) {
        console.error('[Service Worker] Failed during activation:', err)
      }
    })()
  )
})

self.addEventListener('fetch', (event: unknown) => {
  if (!(event instanceof FetchEvent)) return

  const url = new URL(event.request.url)

  const shouldHandle = url.pathname.includes('/imported') || url.pathname.endsWith('.mp4')

  if (!shouldHandle) return

  event.respondWith(handleMediaRequest(event.request))
})

async function handleMediaRequest(request: Request): Promise<Response> {
  const url = new URL(request.url)
  const parts = url.pathname.split('/') // e.g., ['', 'media-proxy', 'imported', 'abc123.mp4']
  const filename = parts.at(-1) ?? ''
  const hash = filename.replace(/\.[^.]+$/, '') // remove extension
  console.log('intercepting: hash is', hash)

  const token = await getHashToken(hash)

  if (typeof token !== 'string' || token.trim() === '') {
    console.error('Token is missing or invalid:', token)
    return new Response('Unauthorized', { status: 401 })
  }

  // Inject the Authorization header into the original request headers
  const headers = new Headers(request.headers)
  headers.set('Authorization', `Bearer ${token}`)

  // Only override the mode and headers to preserve all other browser-generated settings (e.g., Range)
  const modifiedRequest = new Request(request, {
    mode: 'cors', // Use 'cors' instead if cross-origin requests are needed
    headers
  })
  return fetch(modifiedRequest)
}
