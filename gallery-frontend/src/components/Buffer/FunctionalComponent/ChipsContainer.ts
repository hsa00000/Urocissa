import { FunctionalComponent, h, Fragment, PropType } from 'vue'
import ProcessingChip from './ProcessingChip'
import DurationChip from './DurationChip'
import AlbumChip from './AlbumChip'
import FilenameChip from './FilenameChip'
import { AbstractData, DisplayElement } from '@type/types'
import { formatDuration } from '@utils/dater'
import { basename, extname } from 'upath'
import { useConstStore } from '@/store/constStore'

interface ChipsContainerProps {
  abstractData: AbstractData
  displayElement: DisplayElement
}

const ChipsContainer: FunctionalComponent<ChipsContainerProps> = (props) => {
  const chips = []
  const data = props.abstractData.data
  const maxWidth = `${(props.displayElement.displayWidth - 16) * 0.75}px`
  const constStore = useConstStore('mainId')
  if (data.type === 'image' || data.type === 'video') {
    if (data.pending) {
      chips.push(h(ProcessingChip))
    }

    if (data.type === 'video' && data.duration !== undefined) {
      const formattedDuration = formatDuration(String(data.duration))
      chips.push(h(DurationChip, { label: formattedDuration }))
    }

    if (constStore.showFilenameChip) {
      const file = props.abstractData.alias[0]?.file
      if (file) {
        const base = basename(file)
        const filename = basename(base, extname(base))
        chips.push(h(FilenameChip, { label: filename, maxWidth: maxWidth }))
      }
    }

    return h(Fragment, null, chips)
  }

  const albumTitle = data.title

  chips.push(
    h(AlbumChip, {
      label: albumTitle ?? 'Untitled',
      maxWidth: maxWidth
    })
  )

  // Return all chips wrapped in a Fragment
  return h(Fragment, null, chips)
}

// Define the props for the component with type safety
ChipsContainer.props = {
  abstractData: {
    type: Object as PropType<AbstractData>,
    required: true
  },
  displayElement: {
    type: Object as PropType<DisplayElement>,
    required: true
  }
}

export default ChipsContainer
