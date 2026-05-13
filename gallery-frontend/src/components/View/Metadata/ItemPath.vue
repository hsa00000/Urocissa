<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon>mdi-folder</v-icon>
      </v-avatar>
    </template>
    <v-list-item-title :class="metadataTextClass">{{ filePath }}</v-list-item-title>
    <v-list-item-subtitle :class="metadataTextClass">{{
      `${filePathComplete}`
    }}</v-list-item-subtitle>
  </v-list-item>
</template>

<script setup lang="ts">
import type { GalleryImage, GalleryVideo } from '@type/types'
import { computed } from 'vue'
import * as upath from 'upath'
import { useMetadataItemLayout } from './useMetadataItemLayout'

const props = defineProps<{
  database: GalleryImage | GalleryVideo
}>()

const { metadataTextClass } = useMetadataItemLayout()

const filePathComplete = computed(() => {
  return props.database.alias[0]?.file
})

const filePath = computed(() => {
  if (filePathComplete.value != null) {
    const basename = upath.basename(filePathComplete.value)
    return upath.basename(basename, upath.extname(basename))
  }
  return ''
})
</script>
