import { watch, type Ref, type ComputedRef } from 'vue'
import { useInitializedStore } from '@/store/initializedStore'
import { useDataLengthStore } from '@/store/dataLengthStore'
import { fixedBigRowHeight, layoutBatchNumber } from '@/script/common/commonType'
import { fetchRowInWorker } from '@/script/inWorker/fetchRowInWorker'

/**
 * Initializes scroll position and client height for the image container.
 * If `locateTo` is set, scrolls to the target row and fetches data.
 *
 * @param imageContainerRef - Image container element reference.
 * @param scrollTop - Current scroll position reference.
 * @param bufferHeight - Buffer height reference.
 * @param lastScrollTop - Last scroll position reference.
 * @param clientHeight - Client height reference.
 */
export function useInitializeScrollPosition(
  imageContainerRef: Ref<HTMLElement | null>,
  scrollTop: Ref<number>,
  bufferHeight: ComputedRef<number>,
  lastScrollTop: Ref<number>,
  clientHeight: Ref<number>,
  windowWidth: Ref<number>
): void {
  const initializedStore = useInitializedStore()
  const dataLengthStore = useDataLengthStore()

  watch(
    // Here windowWidth is watched for the case that when resizing,
    // the imageContainer.scrollTop may be reset to 0 (whenever bufferHeight becomes 0).
    [() => initializedStore.initialized, windowWidth],

    async () => {
      const imageContainer = imageContainerRef.value
      if (imageContainer !== null && initializedStore.initialized) {
        imageContainer.scrollTop = bufferHeight.value / 3

        lastScrollTop.value = bufferHeight.value / 3

        clientHeight.value = imageContainer.clientHeight

        const jumpTo = dataLengthStore.locateTo
        if (jumpTo !== null) {
          const targetRowIndex = Math.floor(jumpTo / layoutBatchNumber)
          scrollTop.value = targetRowIndex * fixedBigRowHeight
          fetchRowInWorker(targetRowIndex)
          dataLengthStore.locateTo = null
        }
      }
    },
    { immediate: true, flush: 'post' }
  )
}
