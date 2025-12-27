<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon>mdi-image</v-icon>
      </v-avatar>
    </template>
    <v-list-item-title class="text-wrap">{{
      `${props.database.width} \u00D7 ${props.database.height}`
    }}</v-list-item-title>
    <v-list-item-subtitle class="text-wrap">{{
      filesize(props.database.size)
    }}</v-list-item-subtitle>
  </v-list-item>
</template>

<script setup lang="ts">
import { filesize } from 'filesize'
import { GalleryImage, GalleryVideo } from '@type/types'

const props = defineProps<{
  database: GalleryImage | GalleryVideo
}>()
</script>
