import { FunctionalComponent, h, PropType } from 'vue'

interface MobileSmallImageProps {
  hasBorder: boolean
  src: string
  onPointerdown: (event: PointerEvent) => void
  onPointerup: (event: PointerEvent) => void
  onPointerleave: () => void
}

const MobileSmallImage: FunctionalComponent<MobileSmallImageProps> = (props) => {
  return h('img', {
    // Prevent the default context menu from appearing
    onContextmenu: (event: Event) => {
      event.preventDefault()
    },
    onPointerdown: props.onPointerdown,
    onPointerup: props.onPointerup,
    onPointerleave: props.onPointerleave,

    style: {
      zIndex: 2,
      position: 'absolute',
      objectFit: 'cover',
      border: props.hasBorder ? '8px solid white' : undefined
    },
    class: 'mobile-small-image w-100 h-100',
    src: props.src
  })
}

MobileSmallImage.props = {
  hasBorder: {
    type: Boolean,
    required: true
  },
  src: {
    type: String,
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
  }
}

export default MobileSmallImage
