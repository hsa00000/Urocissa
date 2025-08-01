<template>
  <AppBar />
  <ProgessBar isolation-id="mainId" />
  <v-navigation-drawer v-model="showDrawer" temporary touchless width="150" class="no-select">
    <v-list nav :key="route.fullPath" :disabled="!initializedStore.initialized">
      <v-list-item slim to="/home" prepend-icon="mdi-home" title="Home"></v-list-item>
      <v-divider></v-divider>
      <v-list-item slim to="/favorite" prepend-icon="mdi-star" title="Favorite"></v-list-item>
      <v-list-item
        slim
        to="/archived"
        prepend-icon="mdi-archive-arrow-down"
        title="Archived"
      ></v-list-item>
      <v-list-item slim to="/trashed" prepend-icon="mdi-trash-can" title="Trashed"></v-list-item>
      <v-list-item slim to="/all" prepend-icon="mdi-expand-all" title="All"></v-list-item>
      <v-divider></v-divider>
      <v-list-item slim to="/albums" prepend-icon="mdi-image-album" title="Albums"></v-list-item>
      <v-list-item
        slim
        to="/videos"
        prepend-icon="mdi-play-circle-outline"
        title="Videos"
      ></v-list-item>
      <v-divider></v-divider>
      <v-list-item slim to="/tags" prepend-icon="mdi-tag-multiple" title="Tags"></v-list-item>
      <v-list-item slim to="/links" prepend-icon="mdi-link" title="Links"></v-list-item>
      <v-divider></v-divider>
      <v-slider
        v-model="constStore.subRowHeightScale"
        :min="50"
        :max="450"
        :step="1"
        :disabled="!initializedStore.initialized"
        density="compact"
        hide-details
        thumb-size="16"
        class="my-2"
      >
        <template #prepend>
          <v-icon icon="mdi-minus" size="x-small"></v-icon>
        </template>
        <template #append>
          <v-icon icon="mdi-plus" size="x-small"></v-icon>
        </template>
      </v-slider>
    </v-list>
  </v-navigation-drawer>
  <EditTagsModal v-if="modalStore.showEditTagsModal" />
  <EditAlbumsModal v-if="modalStore.showEditAlbumsModal" />
  <EditBatchTagsModal v-if="modalStore.showBatchEditTagsModal" />
  <EditBatchAlbumsModal v-if="modalStore.showBatchEditAlbumsModal" />
  <UploadModal v-if="modalStore.showUploadModal" />
</template>

<script setup lang="ts">
import EditTagsModal from '@/components/Modal/EditTagsModal.vue'
import EditBatchTagsModal from '@/components/Modal/EditBatchTagsModal.vue'
import AppBar from '@/components/NavBar/AppBar.vue'
import UploadModal from '@/components/Modal/UploadModal.vue'
import EditAlbumsModal from '@/components/Modal/EditAlbumsModal.vue'
import EditBatchAlbumsModal from '@/components/Modal/EditBatchAlbumsModal.vue'
import ProgessBar from './ProgessBar.vue'
import { useRoute } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { provide, ref } from 'vue'
import { useInitializedStore } from '@/store/initializedStore'
import { useConstStore } from '@/store/constStore'
const showDrawer = ref(false)
const route = useRoute()
const modalStore = useModalStore('mainId')
const initializedStore = useInitializedStore('mainId')
const constStore = useConstStore('mainId')
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
