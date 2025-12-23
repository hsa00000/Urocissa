const DB_NAME = 'hashToken'
const DB_VERSION = 2
const HASH_STORE_NAME = 'hashToken'
const SHARE_STORE_NAME = 'shareInfo'

// Export constants for Service Worker
export { DB_NAME, DB_VERSION, SHARE_STORE_NAME }

function openHashDB(): Promise<IDBDatabase | null> {
  return new Promise((resolve) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION)

    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result
      if (!db.objectStoreNames.contains(HASH_STORE_NAME)) {
        db.createObjectStore(HASH_STORE_NAME)
      }
      if (!db.objectStoreNames.contains(SHARE_STORE_NAME)) {
        db.createObjectStore(SHARE_STORE_NAME)
      }
    }

    request.onsuccess = (event) => {
      resolve((event.target as IDBOpenDBRequest).result)
    }

    request.onerror = (event) => {
      const error = (event.target as IDBOpenDBRequest).error
      console.error(
        `Database error: ${error instanceof DOMException ? error.message : String(error)}`
      )
      resolve(null)
    }
  })
}

export async function storeHashToken(hash: string, token: string): Promise<void> {
  const db = await openHashDB()
  if (!db) {
    console.error('Failed to open database for storing hash token')
    return
  }

  return new Promise<void>((resolve) => {
    const transaction = db.transaction(HASH_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(HASH_STORE_NAME)
    const request = store.put(token, hash)

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      console.error('Error storing hash token')
      resolve()
    }
  })
}

export async function getHashToken(hash: string): Promise<string | null> {
  const db = await openHashDB()
  if (!db) {
    console.error('Failed to open database for retrieving hash token')
    return null
  }

  return new Promise<string | null>((resolve) => {
    const transaction = db.transaction(HASH_STORE_NAME, 'readonly')
    const store = transaction.objectStore(HASH_STORE_NAME)
    const request = store.get(hash)

    request.onsuccess = () => {
      const rawResult: unknown = request.result
      if (typeof rawResult === 'string') {
        resolve(rawResult)
      } else {
        resolve(null)
      }
    }

    request.onerror = () => {
      console.error('Error retrieving hash token')
      resolve(null)
    }
  })
}

export async function deleteHashToken(hash: string): Promise<void> {
  const db = await openHashDB()
  if (!db) {
    console.error('Failed to open database for deleting hash token')
    return
  }

  return new Promise<void>((resolve) => {
    const transaction = db.transaction(HASH_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(HASH_STORE_NAME)
    const request = store.delete(hash)

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      console.error('Error deleting hash token')
      resolve()
    }
  })
}

// Share info storage for Service Worker
export interface ShareInfo {
  albumId: string | null
  shareId: string | null
  password: string | null
}

// Use composite key: albumId_shareId to support multiple shares
function getShareKey(albumId: string, shareId: string): string {
  return `${albumId}_${shareId}`
}

export async function storeShareInfo(info: ShareInfo): Promise<void> {
  if (!info.albumId || !info.shareId) {
    console.error('Cannot store share info without albumId and shareId')
    return
  }

  const db = await openHashDB()
  if (!db) {
    console.error('Failed to open database for storing share info')
    return
  }

  const key = getShareKey(info.albumId, info.shareId)

  return new Promise<void>((resolve) => {
    const transaction = db.transaction(SHARE_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(SHARE_STORE_NAME)
    const request = store.put(info, key)

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      console.error('Error storing share info')
      resolve()
    }
  })
}

export async function getShareInfo(albumId: string, shareId: string): Promise<ShareInfo | null> {
  const db = await openHashDB()
  if (!db) {
    console.error('Failed to open database for retrieving share info')
    return null
  }

  const key = getShareKey(albumId, shareId)

  return new Promise<ShareInfo | null>((resolve) => {
    const transaction = db.transaction(SHARE_STORE_NAME, 'readonly')
    const store = transaction.objectStore(SHARE_STORE_NAME)
    const request = store.get(key)

    request.onsuccess = () => {
      const result = request.result as ShareInfo | undefined
      resolve(result ?? null)
    }

    request.onerror = () => {
      console.error('Error retrieving share info')
      resolve(null)
    }
  })
}

export async function clearShareInfo(albumId: string, shareId: string): Promise<void> {
  const db = await openHashDB()
  if (!db) {
    console.error('Failed to open database for clearing share info')
    return
  }

  const key = getShareKey(albumId, shareId)

  return new Promise<void>((resolve) => {
    const transaction = db.transaction(SHARE_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(SHARE_STORE_NAME)
    const request = store.delete(key)

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      console.error('Error clearing share info')
      resolve()
    }
  })
}
