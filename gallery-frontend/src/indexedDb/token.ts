const DB_NAME = 'token'
const STORE_NAME = 'store'
const KEY = 'timestampToken'

function openDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, 1)

    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME)
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

export async function storeToken(value: string): Promise<void> {
  const db = await openDB()
  return new Promise<void>((resolve, reject) => {
    const transaction = db.transaction(STORE_NAME, 'readwrite')
    const store = transaction.objectStore(STORE_NAME)
    const request = store.put(value, KEY)

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      reject(new Error('Error storing token'))
    }
  })
}

export async function getToken(): Promise<string | null> {
  const db = await openDB()
  return new Promise<string | null>((resolve, reject) => {
    const transaction = db.transaction(STORE_NAME, 'readonly')
    const store = transaction.objectStore(STORE_NAME)
    const request = store.get(KEY)

    request.onsuccess = () => {
      resolve(request.result as string | null)
    }

    request.onerror = () => {
      reject(new Error('Error retrieving token'))
    }
  })
}

export async function deleteToken(): Promise<void> {
  const db = await openDB()
  return new Promise<void>((resolve, reject) => {
    const transaction = db.transaction(STORE_NAME, 'readwrite')
    const store = transaction.objectStore(STORE_NAME)
    const request = store.delete(KEY)

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      reject(new Error('Error deleting token'))
    }
  })
}
