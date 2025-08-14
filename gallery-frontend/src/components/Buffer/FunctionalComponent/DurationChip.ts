import { FunctionalComponent, h } from 'vue'
import { ZIndex } from '@/constants/zIndex'
import { VChip } from 'vuetify/components'

interface DurationChipProps {
  label: string
}

const DurationChip: FunctionalComponent<DurationChipProps> = (props) => {
  return h(
    VChip,
    {
      id: 'duration-chip',
      density: 'comfortable',
      size: 'small',
      color: 'black',
      variant: 'flat',
      prependIcon: 'mdi-play-circle-outline',
      class: 'position-absolute ma-2',
      style: {
        bottom: '0px',
        right: '0px',
        zIndex: ZIndex.componentOverlay
      }
    },
    () => props.label
  )
}

DurationChip.props = {
  label: {
    type: String,
    required: true
  }
}

export default DurationChip
