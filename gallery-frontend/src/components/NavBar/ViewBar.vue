<template>
  <v-toolbar class="position-absolute my-toolbar">
    <LeaveView />
    <v-spacer></v-spacer>
    <ShowInfo />
    <template v-if="route.meta.baseName !== 'share'">
      <v-btn
        v-if="metadata && metadata.database"
        :icon="metadata.database.tag.includes('_favorite') ? 'mdi-star' : 'mdi-star-outline'"
        @click="
          metadata.database.tag.includes('_favorite')
            ? quickRemoveTags('_favorite', [index], isolationId)
            : quickAddTags('_favorite', [index], isolationId)
        "
      ></v-btn>
      <v-btn
        v-if="metadata && metadata.database"
        :icon="
          metadata.database.tag.includes('_archived')
            ? 'mdi-archive-arrow-up-outline'
            : 'mdi-archive-arrow-down-outline'
        "
        @click="
          metadata.database.tag.includes('_archived')
            ? quickRemoveTags('_archived', [index], isolationId)
            : quickAddTags('_archived', [index], isolationId)
        "
      ></v-btn>
    </template>
    <DatabaseMenu
      v-if="metadata && metadata.database && share === null"
      :database="metadata.database"
      :index="index"
      :hash="hash"
      :isolation-id="isolationId"
    />
    <ShareMenu
      v-if="metadata && metadata.database && share !== null"
      :database="metadata.database"
      :index="index"
      :hash="hash"
      :isolation-id="isolationId"
    />
    <AlbumMenu
      v-if="metadata && metadata.album"
      :album="metadata.album"
      :index="index"
      :hash="hash"
      :isolation-id="isolationId"
    />
  </v-toolbar>
</template>
<script setup lang="ts">
import { quickRemoveTags, quickAddTags } from '@utils/quickEditTags'
import { AbstractData, IsolationId } from '@type/types'
import DatabaseMenu from '@Menu/Page/SingleMenu.vue'
import AlbumMenu from '@Menu/Page/AlbumMenu.vue'
import ShareMenu from '@Menu/Page/ShareMenu.vue'
import LeaveView from '@Menu/MenuButton/BtnLeaveView.vue'
import ShowInfo from '@Menu/MenuButton/BtnShowInfo.vue'
import { useRoute } from 'vue-router'
import { usePrefetchStore } from '@/store/prefetchStore'

const route = useRoute()
const prefetchStore = usePrefetchStore('mainId')
const share = prefetchStore.share

defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  metadata: AbstractData | undefined
}>()
</script>
