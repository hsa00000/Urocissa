self.addEventListener('fetch', (event: unknown) => {
  if (!(event instanceof FetchEvent)) {
    return
  }

  const url = new URL(event.request.url)

  if (url.pathname.startsWith('/object')) {
    event.respondWith(
      (async () => {
        const modifiedHeaders = new Headers(event.request.headers)
        modifiedHeaders.set('Authorization', 'Bearer YOUR_AUTH_TOKEN')

        const modifiedRequest = new Request(event.request, {
          headers: modifiedHeaders
        })

        return fetch(modifiedRequest)
      })()
    )
  } else {
    // Let other requests pass through
    console.log('[Service Worker] Fetching:', event.request.url)
    event.respondWith(fetch(event.request))
  }
})
