import { ref } from 'vue'
import { useRouter, isNavigationFailure, NavigationFailureType } from 'vue-router'
import { createNonEmptyAlbum } from '@utils/createAlbums'
import { navigateToAlbum } from '@/route/navigator'
import type { IsolationId } from '@type/types'
import type { SelectionInput } from '@/type/selection'

export function useCreateAlbumAction() {
  const loading = ref(false)
  const router = useRouter()

  const createAndNavigate = async (
    elementsIndex: SelectionInput,
    isolationId: IsolationId,
    onSuccess: () => void
  ) => {
    loading.value = true
    try {
      const newAlbumId = await createNonEmptyAlbum(elementsIndex, isolationId)

      if (typeof newAlbumId === 'string' && newAlbumId.length > 0) {
        // Close modals or clear states before navigation
        onSuccess()

        try {
          const failure = await navigateToAlbum(newAlbumId, router)

          if (isNavigationFailure(failure, NavigationFailureType.aborted)) {
            console.warn('Navigation aborted:', failure)
          } else if (isNavigationFailure(failure, NavigationFailureType.cancelled)) {
            console.warn('Navigation cancelled:', failure)
          }
        } catch (err) {
          console.error('navigateToAlbum threw:', err)
        }
      }
    } finally {
      loading.value = false
    }
  }

  return {
    loading,
    createAndNavigate
  }
}
