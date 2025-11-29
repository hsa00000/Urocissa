<template>
  <slot name="custom-bar">
    <HomeMainBar />
  </slot>
  <Drawer />
  <slot name="content"></slot>
</template>

<script setup lang="ts">
import HomeMainBar from '@/components/NavBar/HomeBars/HomeMainBar.vue'
import Drawer from './Drawer.vue'
import { provide, ref, onMounted, onUnmounted } from 'vue'
import { useCollectionStore } from '@/store/collectionStore'
import { onBeforeRouteLeave } from 'vue-router'

const showDrawer = ref(false)
const collectionStore = useCollectionStore('mainId')

provide('showDrawer', showDrawer)

const exitEditMode = () => {
  if (collectionStore.editModeOn) {
    collectionStore.editModeOn = false
    return true
  }
  return false
}

const handleKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Escape') {
    collectionStore.leaveEdit()
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
})

onBeforeRouteLeave(() => {
  // 如果有從編輯模式退回，就取消這次導航
  if (exitEditMode()) {
    return false
  }
  // 沒在編輯模式，就直接放行（不 return 任何東西）
})
</script>
