import { FunctionalComponent, h } from 'vue'
import { VChip } from 'vuetify/components'

interface FilenameChipProps {
  label: string
  maxWidth: string
}

const FilenameChip: FunctionalComponent<FilenameChipProps> = (props) => {
  return h(
    VChip,
    {
      id: 'filename-chip',
      density: 'comfortable',
      size: 'small',
      color: 'black',
      variant: 'flat',
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

FilenameChip.props = {
  label: {
    type: String,
    required: true
  },
  maxWidth: {
    type: String,
    required: true
  }
}

export default FilenameChip