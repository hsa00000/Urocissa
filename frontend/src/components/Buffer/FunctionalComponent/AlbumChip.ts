import { FunctionalComponent, h } from 'vue'
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
      color: 'black',
      variant: 'flat',
      prependIcon: 'mdi-image-album',
      class: 'position-absolute ma-2',
      style: {
        bottom: '0px',
        right: '0px',
        zIndex: 4
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
