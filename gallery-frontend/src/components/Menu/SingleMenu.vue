<template>
  <v-menu location="start">
    <template #activator="{ props: MenuBtn }">
      <v-btn v-bind="MenuBtn" icon="mdi-dots-vertical"></v-btn>
    </template>
    <v-list>
      <ItemViewOriginalFile
        v-if="shareStore.resolvedShare === null || shareStore.resolvedShare.share.showDownload"
        :src="getSrc(database.hash, true, database.ext, tokenStore.hashTokenMap.get(database.hash))"
        :isolation-id="props.isolationId"
        :hash="database.hash"
      />
      <ItemDownload
        v-if="shareStore.resolvedShare === null || shareStore.resolvedShare.share.showDownload"
        :index-list="[props.index]"
      />
      <ItemFindInTimeline :hash="props.hash" />
      <v-divider></v-divider>
      <ItemEditTags />
      <ItemEditAlbums />
      <ItemDelete v-if="!database.tags.includes('_trashed')" :index-list="[props.index]" />
      <ItemRestore v-if="database.tags.includes('_trashed')" :index-list="[props.index]" />
      <ItemPermanentlyDelete v-if="database.tags.includes('_trashed')" :index-list="[props.index]" />
      <v-divider></v-divider>
      <ItemRegenerateMetadata :index-list="[props.index]" />
      <ItemRegenerateThumbnailByFrame v-if="currentFrameStore.video !== null" />
    </v-list>
  </v-menu>
</template>
<script setup lang="ts">
import { Database, IsolationId } from '@type/types'
import { getSrc } from '@utils/getter'
import { useTokenStore } from '@/store/tokenStore'
import { useShareStore } from '@/store/shareStore'
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
const tokenStore = useTokenStore(props.isolationId)
const shareStore = useShareStore('mainId')
</script>
