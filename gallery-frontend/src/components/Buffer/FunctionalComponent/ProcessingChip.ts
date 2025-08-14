import { FunctionalComponent, h } from 'vue'
import { ZIndex } from '@/constants/zIndex'
import { VChip } from 'vuetify/components'

const ProcessingChip: FunctionalComponent = () => {
  return h(
    VChip,
    {
      id: 'processing-chip',
      prependIcon: 'mdi-alert-circle-outline',
      density: 'comfortable',
      size: 'small',
      color: 'grey',
      variant: 'flat',
      class: 'position-absolute ma-2',
      style: {
        top: '0px',
        right: '0px',
        zIndex: ZIndex.componentOverlay
      }
    },
    'Processing'
  )
}

export default ProcessingChip
