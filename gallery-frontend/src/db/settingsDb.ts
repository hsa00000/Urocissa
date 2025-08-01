const DB_NAME = 'uiSettings'
const SETTINGS_STORE_NAME = 'settings'

function openSettingsDB(): Promise<IDBDatabase | null> {
  return new Promise((resolve) => {
    const request = indexedDB.open(DB_NAME, 1)

    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result
      if (!db.objectStoreNames.contains(SETTINGS_STORE_NAME)) {
        db.createObjectStore(SETTINGS_STORE_NAME)
      }
    }

    request.onsuccess = (event) => {
      resolve((event.target as IDBOpenDBRequest).result)
    }

    request.onerror = (event) => {
      const error = (event.target as IDBOpenDBRequest).error
      console.error(
        `Settings database error: ${error instanceof DOMException ? error.message : String(error)}`
      )
      resolve(null)
    }
  })
}

export async function storeSubRowHeightScale(value: number): Promise<void> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for storing subRowHeightScale')
    return
  }

  return new Promise<void>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.put(value, 'subRowHeightScale')

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      console.error('Error storing subRowHeightScale')
      resolve()
    }
  })
}

export async function getSubRowHeightScale(): Promise<number | null> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for retrieving subRowHeightScale')
    return null
  }

  return new Promise<number | null>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readonly')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.get('subRowHeightScale')

    request.onsuccess = () => {
      const rawResult: unknown = request.result
      if (typeof rawResult === 'number') {
        resolve(rawResult)
      } else {
        resolve(null)
      }
    }

    request.onerror = () => {
      console.error('Error retrieving subRowHeightScale')
      resolve(null)
    }
  })
}

export async function deleteSubRowHeightScale(): Promise<void> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for deleting subRowHeightScale')
    return
  }

  return new Promise<void>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.delete('subRowHeightScale')

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      console.error('Error deleting subRowHeightScale')
      resolve()
    }
  })
}
