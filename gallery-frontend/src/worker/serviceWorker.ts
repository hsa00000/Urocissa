import { getHashToken, DB_NAME, DB_VERSION, SHARE_STORE_NAME } from '@/db/db'
import type { ShareInfo } from '@/db/db'

// Extract albumId and shareId from referer URL (e.g., /share/albumId-shareId or /share/albumId-shareId/view/hash)
function extractShareIdsFromReferer(referer: string): {
  albumId: string | null
  shareId: string | null
} {
  try {
    const url = new URL(referer)
    // Match pattern: /share/albumId-shareId (shareId should not contain /)
    const match = url.pathname.match(/\/share\/([^-]+)-([^/]+)/)
    if (match && match[1] && match[2]) {
      return { albumId: match[1], shareId: match[2] }
    }
  } catch {
    // Invalid URL
  }
  return { albumId: null, shareId: null }
}

// Get share info from IndexedDB using albumId and shareId
async function getShareInfoFromDB(albumId: string, shareId: string): Promise<ShareInfo | null> {
  return new Promise((resolve) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION)

    request.onerror = () => {
      resolve(null)
    }

    request.onsuccess = () => {
      const db = request.result
      try {
        const transaction = db.transaction(SHARE_STORE_NAME, 'readonly')
        const store = transaction.objectStore(SHARE_STORE_NAME)
        const key = `${albumId}_${shareId}`

        const getRequest = store.get(key)

        getRequest.onsuccess = () => {
          resolve(getRequest.result || null)
        }
        getRequest.onerror = () => {
          resolve(null)
        }
      } catch {
        resolve(null)
      }
    }
  })
}

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

  const token = await getHashToken(hash)

  if (typeof token !== 'string' || token.trim() === '') {
    return new Response('Unauthorized', { status: 401 })
  }

  // Inject the Authorization header into the original request headers
  const headers = new Headers(request.headers)
  headers.set('Authorization', `Bearer ${token}`)

  // Extract albumId and shareId from referer to get correct share info
  const { albumId, shareId } = extractShareIdsFromReferer(request.referrer)

  if (albumId && shareId) {
    const shareInfo = await getShareInfoFromDB(albumId, shareId)

    // Add share headers if available
    if (shareInfo) {
      if (shareInfo.albumId) {
        headers.set('x-album-id', shareInfo.albumId)
      }
      if (shareInfo.shareId) {
        headers.set('x-share-id', shareInfo.shareId)
      }
      if (shareInfo.password) {
        headers.set('x-share-password', shareInfo.password)
      }
    }
  }

  // Only override the mode and headers to preserve all other browser-generated settings (e.g., Range)
  const modifiedRequest = new Request(request, {
    mode: 'cors', // Use 'cors' instead if cross-origin requests are needed
    headers
  })
  return fetch(modifiedRequest)
}
