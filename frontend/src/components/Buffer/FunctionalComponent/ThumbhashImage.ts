import { FunctionalComponent, h, Transition } from 'vue'
import { useThemeClasses } from '@/style/useThemeClasses'

interface ThumbhashImageProps {
  src: string | undefined
}

const ThumbhashImage: FunctionalComponent<ThumbhashImageProps> = (props) => {
  const { placeholderBgClass } = useThemeClasses()
  
  return h(Transition, { name: 'slide-fade', appear: true }, () =>
    h('img', {
      style: {
        position: 'absolute',
        zIndex: 1
      },
      class: `thumbhash-image w-100 h-100 ${placeholderBgClass.value}`,
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
