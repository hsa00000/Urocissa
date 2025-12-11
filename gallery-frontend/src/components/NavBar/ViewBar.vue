<template>
  <v-toolbar
    :class="[
      { 'position-absolute': configStore.viewBarOverlay },
      { 'my-toolbar': configStore.viewBarOverlay },
      { 'push-mode': !configStore.viewBarOverlay },
      { 'bg-surface-light': !configStore.viewBarOverlay }
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
        v-if="abstractData && abstractData.data.type !== 'album'"
        :icon="abstractData.data.tags.includes('_favorite') ? 'mdi-star' : 'mdi-star-outline'"
        @click="
          abstractData.data.tags.includes('_favorite')
            ? quickRemoveTags('_favorite', [index], isolationId)
            : quickAddTags('_favorite', [index], isolationId)
        "
      ></v-btn>
      <v-btn
        v-if="abstractData && abstractData.data.type !== 'album'"
        :icon="
          abstractData.data.tags.includes('_archived')
            ? 'mdi-archive-arrow-up-outline'
            : 'mdi-archive-arrow-down-outline'
        "
        @click="
          abstractData.data.tags.includes('_archived')
            ? quickRemoveTags('_archived', [index], isolationId)
            : quickAddTags('_archived', [index], isolationId)
        "
      ></v-btn>
    </template>
    <DatabaseMenu
      v-if="abstractData && abstractData.data.type !== 'album' && share === null"
      :data="abstractData"
      :index="index"
      :hash="hash"
      :isolation-id="isolationId"
    />
    <ShareMenu
      v-if="abstractData && abstractData.data.type !== 'album' && share !== null"
      :data="abstractData"
      :index="index"
      :hash="hash"
      :isolation-id="isolationId"
    />
    <AlbumMenu
      v-if="abstractData && abstractData.data.type === 'album'"
      :album="abstractData.data"
      :index="index"
      :hash="hash"
      :isolation-id="isolationId"
    />
  </v-toolbar>
</template>
<script setup lang="ts">
import { quickRemoveTags, quickAddTags } from '@utils/quickEditTags'
import { AbstractData, IsolationId } from '@type/types'
import DatabaseMenu from '@Menu/SingleMenu.vue'
import AlbumMenu from '@Menu/AlbumMenu.vue'
import ShareMenu from '@Menu/ShareMenu.vue'
import LeaveView from '@Menu/MenuButton/BtnLeaveView.vue'
import ShowInfo from '@Menu/MenuButton/BtnShowInfo.vue'
import { useRoute } from 'vue-router'
import { useShareStore } from '@/store/shareStore'
import { useConfigStore } from '@/store/configStore'

const route = useRoute()
const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  abstractData: AbstractData | undefined
}>()

const shareStore = useShareStore('mainId')
const configStore = useConfigStore(props.isolationId)

const share = shareStore.resolvedShare ?? null
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

.my-toolbar.push-mode {
  position: relative; /* 讓 toolbar 參與排版，會佔據空間 */
}
</style>
