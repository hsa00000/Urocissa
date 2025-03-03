<template>
  <v-toolbar class="position-absolute my-toolbar">
    <LeaveView />
    <v-spacer></v-spacer>
    <ShowInfo />
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
    <ViewPageToolBarDatabase
      v-if="metadata && metadata.database"
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
import { quickRemoveTags, quickAddTags } from '@/script/common/quickEditTags'
import { AbstractData, IsolationId } from '@/script/common/types'
import ViewPageToolBarDatabase from '@Menu//MenuMain/SingleMenu.vue'
import AlbumMenu from '@Menu//MenuMain/AlbumMenu.vue'
import LeaveView from '@Menu/MenuButton/BtnLeaveView.vue'
import ShowInfo from '@Menu/MenuButton/BtnShowInfo.vue'

defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  metadata: AbstractData | undefined
}>()
</script>
