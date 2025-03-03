export function dater(timestamp: number): string {
  const locale = navigator.language
  return new Intl.DateTimeFormat(locale, {
    year: 'numeric',
    month: 'long',
    day: 'numeric'
  }).format(timestamp)
}

export function formatDuration(durationString: string) {
  // Convert the duration string to a number and truncate to the integer part
  const durationInSeconds = Math.floor(parseFloat(durationString))

  // Calculate hours, minutes, and seconds
  const hours = Math.floor(durationInSeconds / 3600)
  const minutes = Math.floor((durationInSeconds % 3600) / 60)
  const seconds = durationInSeconds % 60

  // Determine the formatted duration based on the presence of hours, minutes, and seconds
  let formattedDuration = ''
  if (hours > 0) {
    formattedDuration = `${hours}:${minutes.toString().padStart(2, '0')}:${seconds
      .toString()
      .padStart(2, '0')}`
  } else {
    formattedDuration = `${minutes.toString().padStart(2, '0')}:${seconds
      .toString()
      .padStart(2, '0')}`
  }

  return formattedDuration
}
