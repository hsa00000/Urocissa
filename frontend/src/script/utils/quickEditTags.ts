import { editTags } from '@/api/editTags'
import { IsolationId } from '@type/types'

export async function quickAddTags(tag: string, indexList: number[], isolationId: IsolationId) {
  const indexArray = indexList
  const addTagsArray: string[] = [tag]
  const removeTagsArray: string[] = []
  await editTags(indexArray, addTagsArray, removeTagsArray, isolationId)
}

export async function quickRemoveTags(tag: string, indexList: number[], isolationId: IsolationId) {
  const indexArray = indexList
  const addTagsArray: string[] = []
  const removeTagsArray: string[] = [tag]
  await editTags(indexArray, addTagsArray, removeTagsArray, isolationId)
}

export async function quickEditTags(
  tag: string,
  indexListAdd: number[],
  indexListRemove: number[],
  isolationId: IsolationId
) {
  await quickAddTags(tag, indexListAdd, isolationId)
  await quickRemoveTags(tag, indexListRemove, isolationId)
}
