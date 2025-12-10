import axios from 'axios'
import { useDataStore } from '@/store/dataStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { AbstractData, IsolationId } from '@type/types'

export async function editUserDefinedDescription(
  abstractData: AbstractData,
  descriptionModelValue: string,
  index: number,
  isolationId: IsolationId
) {
  const dataStore = useDataStore('mainId')

  function getCurrentDescription(): string {
    return abstractData.data.description ?? ''
  }

  const prefetchStore = usePrefetchStore(isolationId)
  const timestamp = prefetchStore.timestamp

  if (getCurrentDescription() !== descriptionModelValue) {
    const description = descriptionModelValue === '' ? null : descriptionModelValue

    await axios.put('/put/set_user_defined_description', {
      index: index,
      description: description,
      timestamp: timestamp
    })

    // Update local data store
    const item = dataStore.data.get(index)
    if (item) {
      item.data.description = description ?? undefined
    }
  }
}
