<template>
  <v-menu location="start">
    <template #activator="{ props: MenuBtn }">
      <v-btn v-bind="MenuBtn" icon="mdi-dots-vertical"></v-btn>
    </template>
    <v-list>
      <ItemViewOriginalFile
        :src="getSrc(database.hash, true, database.ext, Cookies.get('jwt')!, undefined )"
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
import { getSrc } from '@/../config'
import Cookies from 'js-cookie'
import ItemViewOriginalFile from '@Menu/MenuItem/ItemViewOriginalFile.vue'
import ItemDownload from '@Menu/MenuItem/ItemDownload.vue'
import ItemFindInTimeline from '@Menu/MenuItem/ItemFindInTimeline.vue'
import ItemEditTags from '@Menu/MenuItem/ItemEditTags.vue'
import ItemEditAlbums from '@Menu/MenuItem/ItemEditAlbums.vue'
import ItemDelete from '@Menu/MenuItem/ItemDelete.vue'
import ItemPermanentlyDelete from '@Menu/MenuItem/ItemPermanentlyDelete.vue'
import ItemRegenerateMetadata from '@Menu/MenuItem/ItemRegenerateMetadata.vue'
import ItemRestore from '@Menu/MenuItem/ItemRestore.vue'
import ItemRegenerateThumbnailByFrame from '@Menu/MenuItem/ItemRegenerateThumbnailByFrame.vue'
import { useCurrentFrameStore } from '@/store/currentFrameStore'

const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  database: Database
}>()

const currentFrameStore = useCurrentFrameStore(props.isolationId)
</script>
