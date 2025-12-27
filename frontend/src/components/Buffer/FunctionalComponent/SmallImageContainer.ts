import { Fragment, FunctionalComponent, h, PropType } from 'vue'
import DesktopSmallImage from './DesktopSmallImage'
import MobileSmallImage from './MobileSmallImage'
import { EnrichedUnifiedData, DisplayElement, IsolationId } from '@type/types'
import { useImgStore } from '@/store/imgStore'
import { useQueueStore } from '@/store/queueStore'
import { useWorkerStore } from '@/store/workerStore'
import { getArrayValue } from '@utils/getter'
import { useShareStore } from '@/store/shareStore'
import { useTokenStore } from '@/store/tokenStore'
import { useConstStore } from '@/store/constStore'

interface SmallImageContainerProps {
  abstractData: EnrichedUnifiedData
  index: number
  displayElement: DisplayElement
  isolationId: IsolationId
  mobile: boolean
  onPointerdown: (event: PointerEvent) => void
  onPointerup: (event: PointerEvent) => void
  onPointerleave: () => void
  onClick: (event: MouseEvent) => void
}

const SmallImageContainer: FunctionalComponent<SmallImageContainerProps> = (props) => {
  const imgStore = useImgStore(props.isolationId)
  const queueStore = useQueueStore(props.isolationId)

  const src = imgStore.imgUrl.get(props.index)

  if (src === undefined) {
    if (!queueStore.img.has(props.index)) {
      queueStore.img.add(props.index)
      checkAndFetch(
        props.abstractData,
        props.index,
        props.displayElement.displayWidth,
        props.displayElement.displayHeight,
        props.isolationId
      ).catch((err: unknown) => {
        console.error('checkAndFetch failed:', err)
      })
    }
    return null
  }

  const chips = []

  const hasBorder = props.abstractData.type === 'album'

  if (props.mobile) {
    chips.push(
      h(MobileSmallImage, {
        hasBorder: hasBorder,
        src: src,
        onPointerdown: props.onPointerdown,
        onPointerup: props.onPointerup,
        onPointerleave: props.onPointerleave
      })
    )
  } else {
    chips.push(
      h(DesktopSmallImage, {
        hasBorder: hasBorder,
        src: src,
        onClick: props.onClick
      })
    )
  }

  return h(Fragment, null, chips)
}

SmallImageContainer.props = {
  abstractData: {
    type: Object as PropType<EnrichedUnifiedData>,
    required: true
  },
  displayElement: {
    type: Object as PropType<DisplayElement>,
    required: true
  },
  isolationId: {
    type: String as PropType<IsolationId>,
    required: true
  },
  index: {
    type: Number,
    required: true
  },
  mobile: {
    type: Boolean,
    required: true
  },
  onPointerdown: {
    type: Function as PropType<(event: PointerEvent) => void>,
    required: true
  },
  onPointerup: {
    type: Function as PropType<(event: PointerEvent) => void>,
    required: true
  },
  onPointerleave: {
    type: Function as PropType<() => void>,
    required: true
  },
  onClick: {
    type: Function as PropType<(event: MouseEvent) => void>,
    required: true
  }
}
async function checkAndFetch(
  abstractData: EnrichedUnifiedData,
  index: number,
  displayWidth: number,
  displayHeight: number,
  isolationId: IsolationId
) {
  const workerStore = useWorkerStore(isolationId)
  const tokenStore = useTokenStore(isolationId)
  const shareStore = useShareStore('mainId')
  const constStore = useConstStore('mainId')
  const workerIndex = index % constStore.concurrencyNumber

  await tokenStore.refreshTimestampTokenIfExpired()
  const timestampToken = tokenStore.timestampToken
  if (timestampToken === null) {
    throw new Error('timestampToken is null after refresh')
  }

  if (workerStore.postToImgWorkerList === undefined) {
    console.error('workerStore.postToImgWorkerList is undefined')
    return
  }

  if (abstractData.type === 'image' || abstractData.type === 'video') {
    const hash = abstractData.id
    await tokenStore.refreshHashTokenIfExpired(hash)
    const hashToken = tokenStore.hashTokenMap.get(hash)
    if (hashToken === undefined) {
      throw new Error(`hashToken is undefined after refresh for hash: ${hash}`)
    }

    getArrayValue(workerStore.postToImgWorkerList, workerIndex).processSmallImage({
      index,
      hash,
      width: displayWidth,
      height: displayHeight,
      devicePixelRatio: window.devicePixelRatio,
      albumId: shareStore.albumId,
      shareId: shareStore.shareId,
      password: shareStore.password,
      timestampToken,
      hashToken
    })
  } else if (abstractData.type === 'album' && abstractData.cover != null) {
    const hash = abstractData.cover
    await tokenStore.refreshHashTokenIfExpired(hash)
    const hashToken = tokenStore.hashTokenMap.get(hash)
    if (hashToken === undefined) {
      throw new Error(`hashToken is undefined after refresh for cover: ${hash}`)
    }

    getArrayValue(workerStore.postToImgWorkerList, workerIndex).processSmallImage({
      index,
      hash,
      width: displayWidth,
      height: displayHeight,
      devicePixelRatio: window.devicePixelRatio,
      albumMode: true,
      albumId: shareStore.albumId,
      shareId: shareStore.shareId,
      password: shareStore.password,
      timestampToken,
      hashToken
    })
  }
}

export default SmallImageContainer
