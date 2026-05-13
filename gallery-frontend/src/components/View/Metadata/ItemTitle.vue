<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon>mdi-image-album</v-icon>
      </v-avatar>
    </template>
    <v-list-item-title :class="metadataTextClass">
      {{ props.title ?? '(Untitled Album)' }}
    </v-list-item-title>
  </v-list-item>
</template>

<script setup lang="ts">
import { useMetadataItemLayout } from './useMetadataItemLayout'

const props = defineProps<{
  title: string | null
}>()

const { metadataTextClass } = useMetadataItemLayout()
</script>
