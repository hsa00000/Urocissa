<template>
  <v-toolbar
    :class="[
      { 'position-absolute': constStore.viewBarOverlay },
      { 'my-toolbar': constStore.viewBarOverlay },
      { 'push-mode': !constStore.viewBarOverlay },
      { 'bg-surface-light': !constStore.viewBarOverlay }
    ]"
    :style="{
      paddingTop: '2px'
    }"
  >
    <LeaveView />
    <v-spacer></v-spacer>
    <ShowInfo />
    <template v-if="route.meta.baseName !== 'share'">
      <v-btn
        v-if="abstractData && (abstractData.type === 'image' || abstractData.type === 'video')"
        :icon="abstractData.isFavorite ? 'mdi-star' : 'mdi-star-outline'"
        aria-label="Toggle favorite"
        @click="setFavorite([index], !abstractData.isFavorite, isolationId)"
      ></v-btn>
      <v-btn
        v-if="abstractData && (abstractData.type === 'image' || abstractData.type === 'video')"
        :icon="
          abstractData.isArchived
            ? 'mdi-archive-arrow-up-outline'
            : 'mdi-archive-arrow-down-outline'
        "
        aria-label="Toggle archive"
        @click="setArchived([index], !abstractData.isArchived, isolationId)"
      ></v-btn>
    </template>
    <DatabaseMenu
      v-if="
        abstractData &&
        (abstractData.type === 'image' || abstractData.type === 'video') &&
        share === null
      "
      :database="abstractData"
      :index="index"
      :hash="hash"
      :isolation-id="isolationId"
    />
    <ShareMenu
      v-if="
        abstractData &&
        (abstractData.type === 'image' || abstractData.type === 'video') &&
        share !== null &&
        share.showDownload
      "
      :database="abstractData"
      :index="index"
      :hash="hash"
      :isolation-id="isolationId"
    />
    <AlbumMenu
      v-if="abstractData && abstractData.type === 'album'"
      :album="abstractData"
      :index="index"
      :hash="hash"
      :isolation-id="isolationId"
    />
  </v-toolbar>
</template>
<script setup lang="ts">
import { setFavorite, setArchived } from '@/api/editFlags'
import { EnrichedUnifiedData, IsolationId } from '@type/types'
import DatabaseMenu from '@Menu/SingleMenu.vue'
import AlbumMenu from '@Menu/AlbumMenu.vue'
import ShareMenu from '@Menu/ShareMenu.vue'
import LeaveView from '@Menu/MenuButton/BtnLeaveView.vue'
import ShowInfo from '@Menu/MenuButton/BtnShowInfo.vue'
import { useRoute } from 'vue-router'
import { useShareStore } from '@/store/shareStore'
import { onMounted } from 'vue'
import { useConstStore } from '@/store/constStore'

const route = useRoute()
const shareStore = useShareStore('mainId')

defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  abstractData: EnrichedUnifiedData | undefined
}>()

const constStore = useConstStore('mainId')

onMounted(() => {
  constStore.loadViewBarOverlay().catch((error: unknown) => {
    console.error('Failed to load viewBarOverlay:', error)
  })
})


const share = shareStore.resolvedShare?.share ?? null
</script>
<style scoped>
.my-toolbar {
  z-index: 2;
  background: linear-gradient(
    to bottom,
    rgba(0, 0, 0, 0.5) 0%,
    rgba(0, 0, 0, 0.25) 50%,
    rgba(0, 0, 0, 0) 100%
  );
}
</style>
