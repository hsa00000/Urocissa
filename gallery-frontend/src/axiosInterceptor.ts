import axios, { AxiosError, InternalAxiosRequestConfig } from 'axios'
import { errorDisplay } from '@/script/utils/errorDisplay'
import { useShareStore } from '@/store/shareStore'
import { useModalStore } from '@/store/modalStore'
import { useMessageStore } from '@/store/messageStore'
import { useRedirectionStore } from '@/store/redirectionStore'

// --- Constants ---
const HEADERS = {
  ALBUM_ID: 'x-album-id',
  SHARE_ID: 'x-share-id',
  SHARE_PASSWORD: 'x-share-password',
  SILENT_ROUTE_RESOURCE: 'x-silent-route-resource-error'
}

// --- Helpers ---

/**
 * Creates a simple throttler to prevent spamming actions (like 401 redirects).
 */
const createThrottler = (delay: number) => {
  let lastTime = 0
  return () => {
    const now = Date.now()
    if (now - lastTime < delay) return true
    lastTime = now
    return false
  }
}

const isThrottled401 = createThrottler(1000)

/**
 * Aggregates stores to avoid repetitive useStore calls.
 * Note: Must be called inside the interceptor to ensure Pinia is active.
 */
const getStores = () => ({
  shareStore: useShareStore('mainId'),
  modalStore: useModalStore('mainId'),
  messageStore: useMessageStore('mainId'),
  redirectionStore: useRedirectionStore('mainId')
})

// --- Error Handlers ---

/**
 * Handles errors specific to the Public Share / Album context.
 */
function handleShareError(error: AxiosError, stores: ReturnType<typeof getStores>) {
  const { shareStore, modalStore, messageStore } = stores
  const status = error.response?.status

  // 1. Handle 401: Password Required / Stale Request
  if (status === 401) {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unnecessary-condition
    const sentPassword = error.config?.headers?.[HEADERS.SHARE_PASSWORD]
    const currentPassword = shareStore.password

    // Check for "Zombie" requests (stale password sent vs current store state)
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
    if (currentPassword !== null && currentPassword !== undefined && sentPassword !== currentPassword) {
      return // Ignore stale request
    }

    if (!modalStore.showShareLoginModal) {
      shareStore.isLinkExpired = false
      modalStore.showShareLoginModal = true
    }
    return
  }

  // 2. Handle 403: Link Expired / Access Denied
  if (status === 403) {
    const displayMsg = errorDisplay(error)
    messageStore.error(
      displayMsg !== 'Unknown error occurred'
        ? displayMsg
        : 'Share link has expired or access is denied.'
    )

    if (!modalStore.showShareLoginModal) {
      shareStore.isLinkExpired = true
      modalStore.showShareLoginModal = true
    }
  }
}

/**
 * Handles standard application errors (Auth, Permissions, Generic).
 */
async function handleGeneralError(error: AxiosError, stores: ReturnType<typeof getStores>) {
  const { messageStore, redirectionStore } = stores
  const status = error.response?.status

  switch (status) {
    case 401:
      if (isThrottled401()) return // Prevent duplicate snackbars/redirects
      messageStore.error('Session expired or unauthorized. Please login.')
      await redirectionStore.redirectionToLogin()
      break

    case 403: {
      const displayMsg = errorDisplay(error)
      messageStore.error(displayMsg !== 'Unknown error occurred' ? displayMsg : 'Access denied.')
      break
    }

    case 405:
      messageStore.error('Read only mode is on.')
      break

    default: {
      // Generic error handler (400, 404, 500, etc.)
      const displayMsg = errorDisplay(error)
      messageStore.error(displayMsg)
      break
    }
  }
}

/**
 * Main Error Interceptor Logic
 */
async function handleAxiosResponseError(error: AxiosError) {
  if (!error.response) return Promise.reject(error)

  // Route-resource failures are rendered inside the highest matched route layer.
  // Keep authentication handling global, but avoid a duplicate snackbar for all
  // other statuses.
  const silentRouteResource = error.config?.headers.get(HEADERS.SILENT_ROUTE_RESOURCE)
  if (silentRouteResource === 'true' && error.response.status !== 401) {
    return Promise.reject(error)
  }

  const stores = getStores()
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
  const isSharePage = stores.shareStore.albumId !== null && stores.shareStore.albumId !== undefined &&
                      // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
                      stores.shareStore.shareId !== null && stores.shareStore.shareId !== undefined

  if (isSharePage) {
    handleShareError(error, stores)
  } else {
    await handleGeneralError(error, stores)
  }

  return Promise.reject(error)
}

// --- Interceptor Setup ---

export function setupMainAxiosInterceptor() {
  // Request Interceptor
  axios.interceptors.request.use((config: InternalAxiosRequestConfig) => {
    const shareStore = useShareStore('mainId')

    // Only attach share headers if both IDs exist
    if (typeof shareStore.albumId === 'string' && typeof shareStore.shareId === 'string') {
      if (!config.headers.has(HEADERS.ALBUM_ID)) {
        config.headers.set(HEADERS.ALBUM_ID, shareStore.albumId)
      }
      if (!config.headers.has(HEADERS.SHARE_ID)) {
        config.headers.set(HEADERS.SHARE_ID, shareStore.shareId)
      }

      if (
        !config.headers.has(HEADERS.SHARE_PASSWORD) &&
        shareStore.password !== null &&
        shareStore.password !== ''
      ) {
        config.headers.set(HEADERS.SHARE_PASSWORD, shareStore.password)
      }
    }

    return config
  })

  // Response Interceptor
  axios.interceptors.response.use((response) => response, handleAxiosResponseError)
}


