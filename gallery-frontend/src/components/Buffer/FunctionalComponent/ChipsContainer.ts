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
  const database = props.abstractData.database
  const maxWidth = `${(props.displayElement.displayWidth - 16) * 0.75}px`
  const constStore = useConstStore('mainId')
  if (database) {
    const pending = database.pending

    if (pending) {
      chips.push(h(ProcessingChip))
    }
    const duration = database.exifVec.duration

    if (duration !== undefined) {
      const formattedDuration = formatDuration(duration)
      chips.push(h(DurationChip, { label: formattedDuration }))
    }

    const file = database.filename

    if (constStore.showFilenameChip) {
      const base = basename(file)
      const filename = basename(base, extname(base))
      chips.push(h(FilenameChip, { label: filename, maxWidth: maxWidth }))
    }

    return h(Fragment, null, chips)
  }

  const albumTitle = props.abstractData.album?.title

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
