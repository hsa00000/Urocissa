import { FunctionalComponent, h, PropType } from 'vue'
import { VIcon } from 'vuetify/components'

interface DesktopIconWrapperProps {
  onClick: (event: MouseEvent) => void
}

const DesktopHoverIcon: FunctionalComponent<DesktopIconWrapperProps> = (props) => {
  return h(
    'div',
    {
      class: 'child',
      style: {
        position: 'relative'
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
  onClick: {
    type: Function as PropType<(event: MouseEvent) => void>,
    required: true
  }
}

export default DesktopHoverIcon
