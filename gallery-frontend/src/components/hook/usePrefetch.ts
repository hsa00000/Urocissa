import { RouteLocationNormalizedLoadedGeneric } from 'vue-router'
import { watchDebounced } from '@vueuse/core'
import { Ref } from 'vue'
import { IsolationId } from '@/script/common/types'
import { prefetch } from '@/script/prefetch/prefetch'

export function usePrefetch(
  filterJsonString: string | null,
  windowWidth: Ref<number>,
  route: RouteLocationNormalizedLoadedGeneric,
  isolationId: IsolationId
) {
  const stopWatcher = watchDebounced(
    windowWidth,
    async () => {
      if (windowWidth.value > 0) {
        const priorityId = route.query.priorityId as string
        const reverse = route.query.reverse as string
        let locate: string | null = null

        if (route.meta.isViewPage) {
          locate = route.params.hash as string
        } else {
          const queryLocate = route.query.locate
          if (typeof queryLocate === 'string') {
            locate = queryLocate
          }
        }

        await prefetch(filterJsonString, priorityId, reverse, locate)

        stopWatcher() // Stop the watcher after prefetching
      }
    },
    { immediate: true, debounce: 75, maxWait: 1000 }
  )
}
