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

export async function storeShowInfo(value: boolean): Promise<void> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for storing showInfo')
    return
  }

  return new Promise<void>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.put(value, 'showInfo')

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      console.error('Error storing showInfo')
      resolve()
    }
  })
}

export async function getShowInfo(): Promise<boolean | null> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for retrieving showInfo')
    return null
  }

  return new Promise<boolean | null>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readonly')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.get('showInfo')

    request.onsuccess = () => {
      const rawResult: unknown = request.result
      if (typeof rawResult === 'boolean') {
        resolve(rawResult)
      } else {
        resolve(null)
      }
    }

    request.onerror = () => {
      console.error('Error retrieving showInfo')
      resolve(null)
    }
  })
}

export async function deleteShowInfo(): Promise<void> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for deleting showInfo')
    return
  }

  return new Promise<void>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.delete('showInfo')

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      console.error('Error deleting showInfo')
      resolve()
    }
  })
}
export async function storeConcurrencyNumber(value: number): Promise<void> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for storing concurrencyNumber')
    return
  }

  return new Promise<void>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.put(value, 'concurrencyNumber')

    request.onsuccess = () => {
      resolve()
    }
    request.onerror = () => {
      console.error('Error storing concurrencyNumber')
      resolve()
    }
  })
}

export async function getConcurrencyNumber(): Promise<number | null> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for retrieving concurrencyNumber')
    return null
  }

  return new Promise<number | null>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readonly')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.get('concurrencyNumber')

    request.onsuccess = () => {
      const rawResult: unknown = request.result
      if (typeof rawResult === 'number') {
        resolve(rawResult)
      } else {
        resolve(null)
      }
    }

    request.onerror = () => {
      console.error('Error retrieving concurrencyNumber')
      resolve(null)
    }
  })
}

export async function deleteConcurrencyNumber(): Promise<void> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for deleting concurrencyNumber')
    return
  }

  return new Promise<void>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.delete('concurrencyNumber')

    request.onsuccess = () => {
      resolve()
    }
    request.onerror = () => {
      console.error('Error deleting concurrencyNumber')
      resolve()
    }
  })
}

export async function storeLimitRation(value: boolean): Promise<void> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for storing limitRatio')
    return
  }

  return new Promise<void>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.put(value, 'limitRatio')

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      console.error('Error storing limitRatio')
      resolve()
    }
  })
}

export async function getLimitRation(): Promise<boolean | null> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for retrieving limitRatio')
    return null
  }

  return new Promise<boolean | null>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readonly')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.get('limitRatio')

    request.onsuccess = () => {
      const rawResult: unknown = request.result
      if (typeof rawResult === 'boolean') {
        resolve(rawResult)
      } else {
        resolve(null)
      }
    }

    request.onerror = () => {
      console.error('Error retrieving limitRatio')
      resolve(null)
    }
  })
}

export async function deleteLimitRation(): Promise<void> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for deleting limitRatio')
    return
  }

  return new Promise<void>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.delete('limitRatio')

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      console.error('Error deleting limitRatio')
      resolve()
    }
  })
}

// Theme persistence: store string 'dark' or 'light'
export async function storeTheme(value: 'dark' | 'light'): Promise<void> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for storing theme')
    return
  }

  return new Promise<void>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.put(value, 'theme')

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      console.error('Error storing theme')
      resolve()
    }
  })
}

export async function getTheme(): Promise<'dark' | 'light' | null> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for retrieving theme')
    return null
  }

  return new Promise<'dark' | 'light' | null>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readonly')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.get('theme')

    request.onsuccess = () => {
      const rawResult: unknown = request.result
      if (rawResult === 'dark' || rawResult === 'light') {
        resolve(rawResult)
      } else {
        resolve(null)
      }
    }

    request.onerror = () => {
      console.error('Error retrieving theme')
      resolve(null)
    }
  })
}

export async function deleteTheme(): Promise<void> {
  const db = await openSettingsDB()
  if (!db) {
    console.error('Failed to open database for deleting theme')
    return
  }

  return new Promise<void>((resolve) => {
    const transaction = db.transaction(SETTINGS_STORE_NAME, 'readwrite')
    const store = transaction.objectStore(SETTINGS_STORE_NAME)
    const request = store.delete('theme')

    request.onsuccess = () => {
      resolve()
    }

    request.onerror = () => {
      console.error('Error deleting theme')
      resolve()
    }
  })
}
