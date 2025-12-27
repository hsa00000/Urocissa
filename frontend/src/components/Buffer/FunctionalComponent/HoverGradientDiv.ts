import { FunctionalComponent, h } from 'vue'

interface HoverGradientDivProps {
  mobile: boolean
}

const HoverGradientDiv: FunctionalComponent<HoverGradientDivProps> = (props) => {
  if (props.mobile) {
    return null
  }

  return h('div', {
    id: 'hover-gradient-div',
    class: 'position-absolute w-100 child',
    style: {
      zIndex: 3,
      height: '40px',
      background: 'linear-gradient(180deg, rgba(0,0,0,0.5) 0%, rgba(255,255,255,0) 100%)',
      pointerEvents: 'none'
    }
  })
}

HoverGradientDiv.props = {
  mobile: {
    type: Boolean,
    required: true
  }
}

export default HoverGradientDiv
