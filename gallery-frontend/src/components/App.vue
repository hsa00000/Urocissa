<template>
  <v-app
    :class="{ 'no-select': scrollbarStore.isDragging || scrollbarStoreInsideAlbum.isDragging }"
    @dragstart.prevent
    @dragover.prevent
    @drop.prevent
  >
    <v-main class="h-screen">
      <DropZoneModal v-if="!configStore.isMobile" />
      <router-view v-slot="{ Component }" :key="routeKey">
        <component :is="Component" />
      </router-view> </v-main
    ><v-snackbar-queue v-model="messageStore.queue" timeout="2500" />
    <EditTagsModal v-if="modalStore.showEditTagsModal" />
    <EditAlbumsModal v-if="modalStore.showEditAlbumsModal" />
    <EditBatchTagsModal v-if="modalStore.showBatchEditTagsModal" />
    <EditBatchAlbumsModal v-if="modalStore.showBatchEditAlbumsModal" />
    <UploadModal v-if="modalStore.showUploadModal" />
    <SettingModal v-if="modalStore.showSettingModal" />
  </v-app>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
import { computed, onBeforeMount } from 'vue'
import { useScrollbarStore } from '@/store/scrollbarStore'
import { useRerenderStore } from '@/store/rerenderStore'
import { useMessageStore } from '@/store/messageStore'
import DropZoneModal from './Modal/DropZoneModal.vue'
import { useConstStore } from '@/store/constStore'
import isMobile from 'is-mobile'
import { useConfigStore } from '@/store/configStore'
import EditTagsModal from '@/components/Modal/EditTagsModal.vue'
import EditBatchTagsModal from '@/components/Modal/EditBatchTagsModal.vue'
import UploadModal from '@/components/Modal/UploadModal.vue'
import EditAlbumsModal from '@/components/Modal/EditAlbumsModal.vue'
import EditBatchAlbumsModal from '@/components/Modal/EditBatchAlbumsModal.vue'
import SettingModal from '@/components/Modal/SettingModal.vue'
import { useModalStore } from '@/store/modalStore'

const modalStore = useModalStore('mainId')
const scrollbarStore = useScrollbarStore('mainId')
const scrollbarStoreInsideAlbum = useScrollbarStore('subId')
const rerenderStore = useRerenderStore('mainId')
const messageStore = useMessageStore('mainId')
const constStore = useConstStore('mainId')
const configStore = useConfigStore('mainId')
const route = useRoute()

// The routeKey is used to ensure that the router-view reloads the Home.vue component properly.
// Without it, Vue may cache the component for optimization, potentially causing bugs.
const routeKey = computed(() => {
  const currentPage = route.meta.baseName
  const search = typeof route.query.search === 'string' ? route.query.search : ''
  const locate = typeof route.query.locate === 'string' ? route.query.locate : ''
  const priorityId = typeof route.query.priority_id === 'string' ? route.query.priority_id : ''
  const reverse = typeof route.query.reverse === 'string' ? route.query.reverse : ''
  const concurrencyNumber = constStore.concurrencyNumber
  const homeKey = rerenderStore.homeKey.toString()
  return `${currentPage}-${search}-${locate}-${priorityId}-${reverse}-${concurrencyNumber}-${homeKey}`
})

onBeforeMount(async () => {
  await constStore.loadSubRowHeightScale()
  await constStore.loadShowInfo()
  await constStore.loadConcurrencyNumber()
  configStore.isMobile = isMobile()
})
</script>

<style>
/* Disable native dragging on common elements across the app */
img,
a,
svg,
video,
canvas {
  -webkit-user-drag: none;
}

/* Disable text selection only while dragging: applied to the root node with .no-select */
.no-select,
.no-select * {
  user-select: none !important;
  -webkit-user-select: none !important; /* Safari */
  -moz-user-select: none !important; /* Firefox */
  -webkit-touch-callout: none; /* iOS long-press menu */
}

/* Always allow selection for input elements (including Vuetify structures) */
input,
textarea,
[contenteditable='true'],
.v-field__input,
.v-field__input input,
.v-input input,
.v-text-field input {
  user-select: text !important;
  -webkit-user-select: text !important;
  -moz-user-select: text !important;
}

/* Explicitly prevent images and videos from being selectable */
img,
video {
  user-select: none !important;
  -webkit-user-select: none !important;
  -moz-user-select: none !important;
}
</style>
