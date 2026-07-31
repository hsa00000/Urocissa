import { FunctionalComponent, h, PropType } from 'vue'
import { VIcon } from 'vuetify/components'

interface DesktopIconWrapperProps {
  index: number
  left: number
  top: number
  onClick: (event: MouseEvent) => void
}

const DesktopHoverIcon: FunctionalComponent<DesktopIconWrapperProps> = (props) => {
  return h(
    'div',
    {
      class: 'child',
      role: 'button',
      tabindex: 0,
      'aria-label': 'Select item',
      'data-testid': 'select-item',
      'data-item-index': props.index,
      style: {
        position: 'absolute',
        left: 0,
        top: 0,
        width: '40px',
        height: '40px',
        zIndex: 4,
        transform: `translate3d(${props.left}px, ${props.top}px, 0)`
      },
      onClick: props.onClick
    },
    [
      h(VIcon, {
        icon: 'mdi-check-circle',
        style: {
          position: 'absolute',
          margin: '8px',
          zIndex: 4
        }
      })
    ]
  )
}

DesktopHoverIcon.props = {
  index: {
    type: Number,
    required: true
  },
  left: {
    type: Number,
    required: true
  },
  top: {
    type: Number,
    required: true
  },
  onClick: {
    type: Function as PropType<(event: MouseEvent) => void>,
    required: true
  }
}

export default DesktopHoverIcon
