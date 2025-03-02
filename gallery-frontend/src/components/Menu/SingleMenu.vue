<template>
  <v-menu location="start">
    <template #activator="{ props: MenuBtn }">
      <v-btn v-bind="MenuBtn" icon="mdi-dots-vertical"></v-btn>
    </template>
    <v-list>
      <ItemViewOriginalFile
        :src="getSrcWithToken(database.hash, true, database.ext, Cookies.get('jwt')!, undefined, database.token )"
      />
      <ItemDownload :index-list="[props.index]" />
      <ItemFindInTimeline :hash="props.hash" />
      <v-divider></v-divider>
      <ItemEditTags />
      <ItemEditAlbums />
      <ItemDelete v-if="!database.tag.includes('_trashed')" :index-list="[props.index]" />
      <ItemRestore v-if="database.tag.includes('_trashed')" :index-list="[props.index]" />
      <ItemPermanentlyDelete v-if="database.tag.includes('_trashed')" :index-list="[props.index]" />
      <v-divider></v-divider>
      <ItemRegenerateMetadata :index-list="[props.index]" />
      <ItemRegenerateThumbnailByFrame v-if="currentFrameStore.video !== null" />
    </v-list>
  </v-menu>
</template>
<script setup lang="ts">
import { Database, IsolationId } from '@/script/common/types'
import { getSrcWithToken } from '@/worker/utils'
import Cookies from 'js-cookie'
import ItemViewOriginalFile from './Item/ItemViewOriginalFile.vue'
import ItemDownload from './Item/ItemDownload.vue'
import ItemFindInTimeline from './Item/ItemFindInTimeline.vue'
import ItemEditTags from './Item/ItemEditTags.vue'
import ItemEditAlbums from './Item/ItemEditAlbums.vue'
import ItemDelete from './Item/ItemDelete.vue'
import ItemPermanentlyDelete from './Item/ItemPermanentlyDelete.vue'
import ItemRegenerateMetadata from './Item/ItemRegenerateMetadata.vue'
import ItemRestore from './Item/ItemRestore.vue'
import ItemRegenerateThumbnailByFrame from './Item/ItemRegenerateThumbnailByFrame.vue'
import { useCurrentFrameStore } from '@/store/currentFrameStore'

const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  database: Database
}>()

const currentFrameStore = useCurrentFrameStore(props.isolationId)
</script>
