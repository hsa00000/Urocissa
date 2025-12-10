<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon>mdi-image</v-icon>
      </v-avatar>
    </template>
    <v-list-item-title class="text-wrap">{{
      `${props.data.width} x ${props.data.height}`
    }}</v-list-item-title>
    <v-list-item-subtitle class="text-wrap">{{ filesize(props.data.size) }}</v-list-item-subtitle>
  </v-list-item>
</template>

<script setup lang="ts">
import { filesize } from 'filesize'
import type { GalleryImage, GalleryVideo } from '@type/types'

const props = defineProps<{
  data: GalleryImage | GalleryVideo
}>()
</script>
