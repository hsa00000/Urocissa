import type { IsolationId } from '@type/types'
import type { SelectionDescriptor } from '@/type/selection'
import { defineStore } from 'pinia'

export const useCollectionStore = (isolationId: IsolationId) =>
  defineStore('collectionStore' + isolationId, {
    state: (): {
      editModeOn: boolean
      allSelected: boolean
      selectionSet: Set<number>
      lastClick: null | number
    } => ({
      editModeOn: false,
      allSelected: false,
      selectionSet: new Set(),
      lastClick: null
    }),
    actions: {
      leaveEdit() {
        this.clearSelection()
        this.editModeOn = false
      },
      isSelected(index: number) {
        return this.allSelected ? !this.selectionSet.has(index) : this.selectionSet.has(index)
      },
      selectedCount(total: number) {
        return this.allSelected
          ? Math.max(0, total - this.selectionSet.size)
          : this.selectionSet.size
      },
      hasSelection(total: number) {
        return this.selectedCount(total) > 0
      },
      descriptor(): SelectionDescriptor {
        const indices = [...this.selectionSet].sort((left, right) => left - right)
        return this.allSelected
          ? { mode: 'allExcept', excludedIndices: indices }
          : { mode: 'explicit', indices }
      },
      selectAll() {
        this.allSelected = true
        this.selectionSet.clear()
      },
      clearSelection() {
        this.allSelected = false
        this.selectionSet.clear()
        this.lastClick = null
      },
      invertSelection() {
        this.allSelected = !this.allSelected
      },
      addApi(index: number) {
        if (this.allSelected) this.selectionSet.delete(index)
        else this.selectionSet.add(index)
      },
      deleteApi(index: number) {
        if (this.allSelected) this.selectionSet.add(index)
        else this.selectionSet.delete(index)
      },
      singleSelectedIndex(total: number) {
        if (this.selectedCount(total) !== 1) return undefined
        if (!this.allSelected) return this.selectionSet.values().next().value
        for (let index = 0; index < total; index += 1) {
          if (!this.selectionSet.has(index)) return index
        }
        return undefined
      }
    }
  })()
