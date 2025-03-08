const DB_NAME = 'token'
const HASH_STORE_NAME = 'hashToken'

function openHashDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
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
      reject(
        new Error(
          `Database error: ${error instanceof DOMException ? error.message : String(error)}`
        )
      )
    }
  })
}

export async function storeHashedToken(hash: string, token: string): Promise<void> {
  const db = await openHashDB()
  return new Promise<void>((resolve, reject) => {
    const transaction = db.transaction(HASH_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(HASH_STORE_NAME)
    const request = store.put(token, hash)

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      reject(new Error('Error storing hashed token'))
    }
  })
}

export async function getHashedToken(hash: string): Promise<string | null> {
  const db = await openHashDB()
  return new Promise<string | null>((resolve, reject) => {
    const transaction = db.transaction(HASH_STORE_NAME, 'readonly')
    const store = transaction.objectStore(HASH_STORE_NAME)
    const request = store.get(hash)

    request.onsuccess = () => {
      resolve(request.result as string | null)
    }

    request.onerror = () => {
      reject(new Error('Error retrieving hashed token'))
    }
  })
}

export async function deleteHashedToken(hash: string): Promise<void> {
  const db = await openHashDB()
  return new Promise<void>((resolve, reject) => {
    const transaction = db.transaction(HASH_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(HASH_STORE_NAME)
    const request = store.delete(hash)

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      reject(new Error('Error deleting hashed token'))
    }
  })
}
