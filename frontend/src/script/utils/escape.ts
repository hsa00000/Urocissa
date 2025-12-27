export function escapeAndWrap(str: string): string {
  // First, escape backslashes and double quotes in the string
  const escaped = str
    .replace(/\\/g, '\\\\') // Convert \ to \\
    .replace(/"/g, '\\"') // Convert " to \"

  // Wrap the processed string in double quotes
  return `"${escaped}"`
}

export function unescapeAndUnwrap(str: string): string {
  // If the string starts and ends with double quotes, remove them
  if (str.startsWith('"') && str.endsWith('"')) {
    str = str.slice(1, -1)
  }

  // Restore escaped characters
  return str
    .replace(/\\"/g, '"') // Convert \" back to "
    .replace(/\\\\/g, '\\') // Convert \\ back to \
}
