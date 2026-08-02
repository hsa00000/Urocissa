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
    ><!-- Keep persistent snackbars out of the overlay stack so ESC reaches active overlays. -->
    <v-snackbar-queue
      v-model="messageStore.queue"
      timeout="2500"
      :close-on-back="false"
      :_disable-global-stack="true"
    />
    <EditTagsModal v-if="modalStore.showEditTagsModal" />
    <EditAlbumsModal v-if="modalStore.showEditAlbumsModal" />
    <EditBatchTagsModal v-if="modalStore.showBatchEditTagsModal" />
    <EditBatchAlbumsModal v-if="modalStore.showBatchEditAlbumsModal" />
    <UploadModal v-if="modalStore.showUploadModal && route.name !== 'upload'" />
    <SettingModal v-if="modalStore.showSettingModal" />
    <ReindexModal />
  </v-app>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
import { computed, onBeforeMount } from 'vue'
import { useScrollbarStore } from '@/store/scrollbarStore'
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
import ReindexModal from '@/components/Modal/Reindex/ReindexModal.vue'
import { useModalStore } from '@/store/modalStore'
import { createRootRouteKey } from '@/route/rootRouteKey'

const modalStore = useModalStore('mainId')
const scrollbarStore = useScrollbarStore('mainId')
const scrollbarStoreInsideAlbum = useScrollbarStore('subId')
const messageStore = useMessageStore('mainId')
const constStore = useConstStore('mainId')
const configStore = useConfigStore('mainId')
const route = useRoute()

// Collection query changes update in place. Only a different root page or
// worker topology replaces this route host.
const routeKey = computed(() => {
  return createRootRouteKey({
    baseName: route.meta.baseName,
    concurrencyNumber: constStore.concurrencyNumber
  })
})

onBeforeMount(async () => {
  await constStore.loadSubRowHeightScale()
  await constStore.loadLimitRation()
  await constStore.loadShowInfo()
  await constStore.loadConcurrencyNumber()
  await constStore.loadShowFilenameChip()
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
