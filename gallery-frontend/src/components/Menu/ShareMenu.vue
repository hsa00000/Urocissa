<template>
  <v-menu location="start">
    <template #activator="{ props: MenuBtn }">
      <v-btn v-bind="MenuBtn" icon="mdi-dots-vertical"></v-btn>
    </template>
    <v-list>
      <ItemViewOriginalFile
        :src="
          getSrc(
            media.id,
            true,
            media.ext,
            { albumId: shareStore.albumId || null, shareId: shareStore.shareId || null },
            tokenStore.hashTokenMap.get(media.id)
          )
        "
        :hash="media.id"
        :isolation-id="props.isolationId"
      />
      <ItemDownload :index-list="[props.index]" />
    </v-list>
  </v-menu>
</template>
<script setup lang="ts">
import { computed } from 'vue'
import { AbstractData, GalleryImage, GalleryVideo, IsolationId } from '@type/types'
import { getSrc } from '@utils/getter'
import { useTokenStore } from '@/store/tokenStore'
import { useShareStore } from '@/store/shareStore'
import ItemViewOriginalFile from '@Menu/MenuItem/ItemViewOriginalFile.vue'
import ItemDownload from '@Menu/MenuItem/ItemDownload.vue'
const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  data: AbstractData
}>()
const tokenStore = useTokenStore(props.isolationId)
const shareStore = useShareStore('mainId')
const media = computed<GalleryImage | GalleryVideo>(() => {
  if (props.data.data.type === 'album') {
    throw new Error('ShareMenu requires a media item')
  }
  return props.data.data
})
</script>
