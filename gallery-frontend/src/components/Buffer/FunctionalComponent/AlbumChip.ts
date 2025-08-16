import { FunctionalComponent, h } from 'vue'
import { ZIndex } from '@/constants/zIndex'
import { VChip } from 'vuetify/components'

// Define the props interface for AlbumChip
interface AlbumChipProps {
  label: string
  maxWidth: string
}

const AlbumChip: FunctionalComponent<AlbumChipProps> = (props) => {
  return h(
    VChip,
    {
      id: 'album-chip',
      density: 'comfortable',
      size: 'small',
      variant: 'flat',
      prependIcon: 'mdi-image-album',
      class: 'position-absolute ma-2',
      style: {
        bottom: '0px',
        right: '0px',
        zIndex: ZIndex.componentOverlay,
        backgroundColor: 'rgb(var(--v-theme-background))',
        color: 'rgb(var(--v-theme-on-background))'
      }
    },
    () => [
      h(
        'span',
        {
          class: 'text-truncate',
          style: {
            maxWidth: props.maxWidth
          }
        },
        props.label
      )
    ]
  )
}

// Define the props for the component
AlbumChip.props = {
  label: {
    type: String,
    required: true
  },
  maxWidth: {
    type: String,
    required: true
  }
}

export default AlbumChip
