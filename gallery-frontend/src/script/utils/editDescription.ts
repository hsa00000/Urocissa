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
    if (abstractData.database) {
      return abstractData.database.object.description ?? ''
    } else if (abstractData.album) {
      return abstractData.album.object.description ?? ''
    }
    return ''
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
      // [Modify] Update the object description
      if (item.database) {
        item.database.object.description = description
      } else if (item.album) {
        item.album.object.description = description
      }
    }
  }
}
