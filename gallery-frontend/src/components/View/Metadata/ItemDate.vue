<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon>mdi-calendar</v-icon>
      </v-avatar>
    </template>
    <v-list-item-title :class="metadataTextClass">{{ formattedDate }}</v-list-item-title>
    <v-list-item-subtitle :class="metadataTextClass">{{ formattedTime }}</v-list-item-subtitle>
  </v-list-item>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { GalleryImage, GalleryVideo } from '@type/types'
import { dater } from '@utils/dater'
import { useMetadataItemLayout } from './useMetadataItemLayout'

type DatabaseWithTimestamp = (GalleryImage | GalleryVideo) & { timestamp: number }

const props = defineProps<{
  database: DatabaseWithTimestamp
}>()

const { metadataTextClass } = useMetadataItemLayout()

const formattedDate = computed(() => dater(props.database.timestamp))
const formattedTime = computed(() => timer(props.database.timestamp))

function timer(timestamp: number): string {
  const locale = navigator.language
  return new Intl.DateTimeFormat(locale, {
    weekday: 'long',
    hour: 'numeric',
    minute: 'numeric',
    second: 'numeric',
    hour12: true,
    dayPeriod: 'narrow',
    timeZoneName: 'short'
  }).format(timestamp)
}
</script>
