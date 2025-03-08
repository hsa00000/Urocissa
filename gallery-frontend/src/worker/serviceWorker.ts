import { getHashToken } from '@/indexedDb/hashToken'
import { extractHashFromAbsoluteUrl } from '@/script/utils/getter'

self.addEventListener('fetch', (event: unknown) => {
  if (!(event instanceof FetchEvent)) {
    return
  }

  const url = new URL(event.request.url)

  // Early return for non-/object requests
  if (!url.pathname.startsWith('/object')) {
    event.respondWith(fetch(event.request))
    return
  }

  const hash = extractHashFromAbsoluteUrl(url)
  if (hash === null) {
    console.error('[Service Worker] Failed to extract hash from URL:', url.href)
    event.respondWith(new Response('Invalid URL hash', { status: 400 }))
    return
  }

  event.respondWith(
    (async () => {
      try {
        const token = await getHashToken(hash)
        if (token === null) {
          console.error('[Service Worker] Failed to get hash token:', hash)
          return new Response('Failed to get hash token', { status: 404 })
        }
        const modifiedHeaders = new Headers(event.request.headers)
        modifiedHeaders.set('Authorization', `Bearer ${token}`)

        const modifiedRequest = new Request(event.request, {
          headers: modifiedHeaders
        })

        return await fetch(modifiedRequest)
      } catch (error: unknown) {
        console.error('[Service Worker] Failed to get hash token:', error)
        return new Response('Failed to get hash token', { status: 500 })
      }
    })()
  )
})
