import { FunctionalComponent, h, Transition } from 'vue'
import { ZIndex } from '@/constants/zIndex'

interface ThumbhashImageProps {
  src: string | undefined
}

const ThumbhashImage: FunctionalComponent<ThumbhashImageProps> = (props) => {
  return h(Transition, { name: 'slide-fade', appear: true }, () =>
    h('img', {
      style: {
        position: 'absolute',
        zIndex: ZIndex.content
      },
      class: 'thumbhash-image w-100 h-100',
      src: props.src
    })
  )
}

ThumbhashImage.props = {
  src: {
    type: String,
    required: false
  }
}

export default ThumbhashImage
