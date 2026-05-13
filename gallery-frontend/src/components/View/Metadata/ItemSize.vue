<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon>mdi-image</v-icon>
      </v-avatar>
    </template>
    <v-list-item-title :class="metadataTextClass">{{ dimensions }}</v-list-item-title>
    <v-list-item-subtitle :class="metadataTextClass">{{ formattedSize }}</v-list-item-subtitle>
  </v-list-item>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { filesize } from 'filesize'
import type { GalleryImage, GalleryVideo } from '@type/types'
import { useMetadataItemLayout } from './useMetadataItemLayout'

const props = defineProps<{
  database: GalleryImage | GalleryVideo
}>()

const { metadataTextClass } = useMetadataItemLayout()

const dimensions = computed(() => `${props.database.width} \u00D7 ${props.database.height}`)
const formattedSize = computed(() => filesize(props.database.size))
</script>
