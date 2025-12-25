import { DisplayElement, IsolationId } from '@type/types'

import { useDataStore } from '@/store/dataStore'
import { FunctionalComponent, h, PropType } from 'vue'
import ChipsContainer from './ChipsContainer'
import SmallImageContainer from './SmallImageContainer'
import ThumbhashImage from './ThumbhashImage'
import { useConfigStore } from '@/store/configStore'

interface MainBlockProps {
  index: number
  displayElement: DisplayElement
  isolationId: IsolationId
  mobile: boolean
  onPointerdown: (event: PointerEvent) => void
  onPointerup: (event: PointerEvent) => void
  onPointerleave: () => void
  onClick: (event: MouseEvent) => void
}

const MainBlock: FunctionalComponent<MainBlockProps> = (props) => {
  const dataStore = useDataStore(props.isolationId)
  const configStore = useConfigStore(props.isolationId)
  const abstractData = dataStore.data.get(props.index)

  if (!abstractData) {
    return null
  }

  const chips = []
  chips.push(
    h(ChipsContainer, {
      abstractData: abstractData,
      displayElement: props.displayElement
    })
  )

  if (!configStore.disableImg) {
    const thumbhashUrl = abstractData.thumbhashUrl

    if (typeof thumbhashUrl === 'string') {
      chips.push(
        h(ThumbhashImage, {
          index: props.index,
          src: thumbhashUrl
        })
      )
    }

    chips.push(
      h(SmallImageContainer, {
        abstractData: abstractData,
        index: props.index,
        displayElement: props.displayElement,
        isolationId: props.isolationId,
        mobile: props.mobile,
        onPointerdown: props.onPointerdown,
        onPointerup: props.onPointerup,
        onPointerleave: props.onPointerleave,
        onClick: props.onClick
      })
    )
  }
  return h(
    'div',
    {
      class: 'w-100 h-100 position-absolute'
    },
    chips
  )
}

MainBlock.props = {
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

export default MainBlock
