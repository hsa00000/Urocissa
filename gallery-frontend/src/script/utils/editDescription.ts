import axios from 'axios'
import { useDataStore } from '@/store/dataStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { EnrichedUnifiedData, IsolationId } from '@type/types'

export async function editUserDefinedDescription(
  abstractData: EnrichedUnifiedData,
  descriptionModelValue: string,
  index: number,
  isolationId: IsolationId
) {
  const dataStore = useDataStore('mainId')

  function getCurrentDescription(): string {
    // 新結構：description 直接在 data 上
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

    // Update local data store
    const item = dataStore.data.get(index)
    if (item) {
      // 新結構：直接更新 description 屬性
      item.description = descriptionModelValue === '' ? null : descriptionModelValue
    }
  }
}
