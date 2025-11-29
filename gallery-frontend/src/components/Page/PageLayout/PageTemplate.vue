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

const showDrawer = ref(false)
const collectionStore = useCollectionStore('mainId')

provide('showDrawer', showDrawer)

onMounted(() => {
  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Escape' && collectionStore.editModeOn) {
      collectionStore.editModeOn = false
    }
  }
  window.addEventListener('keydown', handleKeydown)
  onUnmounted(() => {
    window.removeEventListener('keydown', handleKeydown)
  })
})
</script>
