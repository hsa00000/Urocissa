import { FunctionalComponent, h } from 'vue'
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
        zIndex: 4
      }
    },
    () => 'Processing'
  )
}

export default ProcessingChip
