import { FunctionalComponent, h, Fragment, PropType } from 'vue'
import ProcessingChip from './ProcessingChip'
import DurationChip from './DurationChip'
import AlbumChip from './AlbumChip'
import { AbstractData, DisplayElement } from '@/script/common/types'
import { formatDuration } from '@utils/dater'

interface ChipsContainerProps {
  abstractData: AbstractData
  displayElement: DisplayElement
}

const ChipsContainer: FunctionalComponent<ChipsContainerProps> = (props) => {
  const chips = []
  const database = props.abstractData.database
  if (database) {
    const pending = database.pending

    if (pending) {
      chips.push(h(ProcessingChip))
    }
    const duration = database.exif_vec.duration

    if (duration !== undefined) {
      const formattedDuration = formatDuration(duration)
      chips.push(h(DurationChip, { label: formattedDuration }))
    }
    return h(Fragment, null, chips)
  }

  const albumTitle = props.abstractData.album?.title
  const maxWidth = `${(props.displayElement.displayWidth - 8) * 0.75}px`

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
