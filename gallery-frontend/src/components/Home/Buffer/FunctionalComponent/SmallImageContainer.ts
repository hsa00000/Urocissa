import { Fragment, FunctionalComponent, h, PropType } from 'vue'
import DesktopSmallImage from './DesktopSmallImage'
import MobileSmallImage from './MobileSmallImage'
import { AbstractData, DisplayElement, IsolationId } from '@/script/common/types'
import { useImgStore } from '@/store/imgStore'
import { useQueueStore } from '@/store/queueStore'
import { useWorkerStore } from '@/store/workerStore'
import { getArrayValue, getCookiesJwt } from '@utils/getter'

interface SmallImageContainerProps {
  abstractData: AbstractData
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
      )
    }
    return null
  }

  const chips = []

  const hasBorder = props.abstractData.album !== undefined

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
    type: Object as PropType<AbstractData>,
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

function checkAndFetch(
  abstractData: AbstractData,
  index: number,
  displayWidth: number,
  displayHeight: number,
  isolationId: IsolationId
) {
  const workerStore = useWorkerStore(isolationId)
  const workerIndex = index % workerStore.concurrencyNumber
  if (workerStore.postToWorkerList !== undefined) {
    if (abstractData.database) {
      getArrayValue(workerStore.postToWorkerList, workerIndex).processSmallImage({
        index: index,
        hash: abstractData.database.hash,
        width: displayWidth,
        height: displayHeight,
        devicePixelRatio: window.devicePixelRatio,
        jwt: getCookiesJwt(),
        token: abstractData.database.token
      })
    } else if (abstractData.album?.cover !== null && abstractData.album?.cover !== undefined) {
      getArrayValue(workerStore.postToWorkerList, workerIndex).processSmallImage({
        index: index,
        hash: abstractData.album.cover,
        width: displayWidth,
        height: displayHeight,
        devicePixelRatio: window.devicePixelRatio,
        jwt: getCookiesJwt(),
        albumMode: true,
        token: abstractData.album.token
      })
    }
  } else {
    console.error('workerStore.postToWorkerList is undefined')
  }
}

export default SmallImageContainer
