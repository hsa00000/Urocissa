import axios from 'axios'
import { useDataStore } from '@/store/dataStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { EnrichedUnifiedData, IsolationId } from '@type/types'
import { updateCachedResource } from './routeResourceCache'

export async function editUserDefinedDescription(
  abstractData: EnrichedUnifiedData,
  descriptionModelValue: string,
  index: number,
  isolationId: IsolationId
) {
  const dataStore = useDataStore(isolationId)

  function getCurrentDescription(): string {
    return abstractData.description ?? ''
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

    const nextDescription = descriptionModelValue === '' ? null : descriptionModelValue
    const item = dataStore.data.get(index)
    if (item) item.description = nextDescription
    updateCachedResource(abstractData.id, (data) => {
      data.description = nextDescription
    })
  }
}
