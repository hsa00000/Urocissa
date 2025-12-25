<template>
  <v-menu location="start">
    <template #activator="{ props: MenuBtn }">
      <v-btn v-bind="MenuBtn" icon="mdi-dots-vertical"></v-btn>
    </template>
    <v-list>
      <ItemViewOriginalFile
        :src="getSrcOriginal(database.id, true, database.ext)"
        :hash="database.id"
        :isolation-id="props.isolationId"
      />
      <ItemDownload :index-list="[props.index]" />
    </v-list>
  </v-menu>
</template>
<script setup lang="ts">
import { GalleryImage, GalleryVideo, IsolationId } from '@type/types'
import { getSrcOriginal } from '@utils/getter'
import ItemViewOriginalFile from '@Menu/MenuItem/ItemViewOriginalFile.vue'
import ItemDownload from '@Menu/MenuItem/ItemDownload.vue'
const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  database: GalleryImage | GalleryVideo
}>()
</script>
