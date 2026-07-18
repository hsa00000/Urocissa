export type SelectionDescriptor =
  | { mode: 'explicit'; indices: number[] }
  | { mode: 'allExcept'; excludedIndices: number[] }

export type SelectionInput = number[] | SelectionDescriptor

export const normalizeSelection = (selection: SelectionInput): SelectionDescriptor =>
  Array.isArray(selection) ? { mode: 'explicit', indices: selection } : selection

export const selectionIncludes = (selection: SelectionDescriptor, index: number): boolean => {
  const values = selection.mode === 'explicit' ? selection.indices : selection.excludedIndices
  const contained = values.includes(index)
  return selection.mode === 'explicit' ? contained : !contained
}

export const createSelectionMatcher = (
  selection: SelectionDescriptor
): ((index: number) => boolean) => {
  const values = new Set(
    selection.mode === 'explicit' ? selection.indices : selection.excludedIndices
  )
  return selection.mode === 'explicit'
    ? (index) => values.has(index)
    : (index) => !values.has(index)
}

export function* selectionBatches(
  selection: SelectionDescriptor,
  total: number,
  batchSize: number
): Generator<number[]> {
  if (!Number.isInteger(batchSize) || batchSize < 1) {
    throw new RangeError('batchSize must be a positive integer')
  }
  let batch: number[] = []
  if (selection.mode === 'explicit') {
    const sorted = [...new Set(selection.indices)]
      .filter((index) => index >= 0 && index < total)
      .sort((left, right) => left - right)
    for (const index of sorted) {
      batch.push(index)
      if (batch.length === batchSize) {
        yield batch
        batch = []
      }
    }
  } else {
    const excluded = new Set(selection.excludedIndices)
    for (let index = 0; index < total; index += 1) {
      if (excluded.has(index)) continue
      batch.push(index)
      if (batch.length === batchSize) {
        yield batch
        batch = []
      }
    }
  }
  if (batch.length > 0) yield batch
}
