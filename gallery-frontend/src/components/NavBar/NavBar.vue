<template>
  <AppBar />
  <ProgessBar isolation-id="mainId" />
  <v-navigation-drawer v-model="showDrawer" temporary touchless width="150" class="no-select">
    <v-list nav :key="route.fullPath">
      <v-list-item slim to="/" prepend-icon="mdi-home" title="Home"></v-list-item>
      <v-list-item slim to="/tags" prepend-icon="mdi-tag-multiple" title="Tags"></v-list-item>
      <v-list-item slim to="/albums" prepend-icon="mdi-image-album" title="Albums"></v-list-item>
      <v-list-item slim to="/favorite" prepend-icon="mdi-star" title="Favorite"></v-list-item>
      <v-list-item
        slim
        to="/archived"
        prepend-icon="mdi-archive-arrow-down"
        title="Archived"
      ></v-list-item>
      <v-list-item slim to="/trashed" prepend-icon="mdi-trash-can" title="Trashed"></v-list-item>
      <v-list-item slim to="/all" prepend-icon="mdi-expand-all" title="All"></v-list-item>
    </v-list>
  </v-navigation-drawer>
  <EditTagsModal v-if="modalStore.showEditTagsModal" />
  <EditAlbumsModal v-if="modalStore.showEditAlbumsModal" />
  <EditBatchTagsModal v-if="modalStore.showBatchEditTagsModal" />
  <EditBatchAlbumsModal v-if="modalStore.showBatchEditAlbumsModal" />
  <UploadModal v-if="uploadStore.uploading" />
  <CreateAlbumsModal v-if="modalStore.showCreateAlbumsModal" />
</template>

<script setup lang="ts">
import EditTagsModal from '@/components/Modal/EditTagsModal.vue'
import EditBatchTagsModal from '@/components/Modal/EditBatchTagsModal.vue'
import AppBar from '@/components/NavBar/AppBar.vue'
import UploadModal from '@/components/Modal/uploadModal.vue'
import CreateAlbumsModal from '@/components/Modal/CreateAlbumsModal.vue'
import EditAlbumsModal from '@/components/Modal/EditAlbumsModal.vue'
import EditBatchAlbumsModal from '@/components/Modal/EditBatchAlbumsModal.vue'
import ProgessBar from './ProgessBar.vue'

import { useRouter, useRoute } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { provide, ref } from 'vue'
import { useUploadStore } from '@/store/uploadStore'
const uploadStore = useUploadStore('mainId')
const showDrawer = ref(false)
const route = useRoute()
const modalStore = useModalStore('mainId')
const router = useRouter()

router.beforeEach((to, from, next) => {
  // Check if the current route or `to` route already includes the dynamic base segment
  // and adjust `to.path` accordingly if it does not
  const currentDynamicBase = extractDynamicBase(from.path)
  const targetDynamicBase = extractDynamicBase(to.path)
  if (!targetDynamicBase && currentDynamicBase) {
    // If the target route does not have a dynamic base but the current route does,
    // prepend it to the target route's path.
    next({ path: `${currentDynamicBase}${to.path}`, query: to.query })
  } else {
    // Proceed with navigation as usual
    next()
  }
})

function extractDynamicBase(path: string) {
  // Return '/share/[id]' if the path includes it
  const match = /\/share\/[a-zA-Z0-9]+/.exec(path)
  return match ? match[0] : ''
}
provide('showDrawer', showDrawer)
</script>

<style scoped>
.no-select {
  user-select: none;
}
.no-select * {
  user-select: none;
}
</style>
