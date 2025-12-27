import { FunctionalComponent, h, PropType } from 'vue'

interface DesktopSmallImageProps {
  hasBorder: boolean
  src: string
  onClick: (event: MouseEvent) => void
}

const DesktopSmallImage: FunctionalComponent<DesktopSmallImageProps> = (props) => {
  return h('img', {
    onClick: props.onClick,
    style: {
      zIndex: 2,
      position: 'absolute',
      objectFit: 'cover',
      border: props.hasBorder ? '8px solid white' : undefined
    },
    class: 'desktop-small-image w-100 h-100',
    src: props.src
  })
}

DesktopSmallImage.props = {
  hasBorder: {
    type: Boolean,
    required: true
  },
  src: {
    type: String,
    required: true
  },
  onClick: {
    type: Function as PropType<(event: MouseEvent) => void>,
    required: true
  }
}

export default DesktopSmallImage
