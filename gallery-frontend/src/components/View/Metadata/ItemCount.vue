<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon>mdi-image</v-icon>
      </v-avatar>
    </template>
    <v-list-item-title :class="metadataTextClass">{{
      `${props.album.itemCount} items`
    }}</v-list-item-title>
    <v-list-item-subtitle :class="metadataTextClass">
      {{ filesize(props.album.itemSize) }}
    </v-list-item-subtitle>
  </v-list-item>
</template>
<script setup lang="ts">
import { filesize } from 'filesize'
import type { GalleryAlbum } from '@type/types'
import { useMetadataItemLayout } from './useMetadataItemLayout'

const props = defineProps<{
  album: GalleryAlbum
}>()

const { metadataTextClass } = useMetadataItemLayout()
</script>
