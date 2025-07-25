const DB_NAME = 'hashToken'
const HASH_STORE_NAME = 'hashToken'

function openHashDB(): Promise<IDBDatabase | null> {
  return new Promise((resolve) => {
    const request = indexedDB.open(DB_NAME, 1)

    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result
      if (!db.objectStoreNames.contains(HASH_STORE_NAME)) {
        db.createObjectStore(HASH_STORE_NAME)
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
