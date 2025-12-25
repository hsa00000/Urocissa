<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon>mdi-folder</v-icon>
      </v-avatar>
    </template>
    <v-list-item-title class="text-wrap">{{ filePath }}</v-list-item-title>
    <v-list-item-subtitle class="text-wrap">{{ `${filePathComplete}` }}</v-list-item-subtitle>
  </v-list-item>
</template>

<script setup lang="ts">
import { GalleryImage, GalleryVideo } from '@type/types'
import { computed } from 'vue'
import * as upath from 'upath'

const props = defineProps<{
  database: GalleryImage | GalleryVideo
}>()

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
