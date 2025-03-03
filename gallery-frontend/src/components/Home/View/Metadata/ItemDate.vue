<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon color="black">mdi-calendar</v-icon>
      </v-avatar>
    </template>
    <v-list-item-title class="text-wrap">{{ dater(props.database.timestamp) }}</v-list-item-title>
    <v-list-item-subtitle class="text-wrap">{{
      timer(props.database.timestamp)
    }}</v-list-item-subtitle>
  </v-list-item>
</template>

<script setup lang="ts">
import { Database } from '@/script/common/types'
import { dater } from '@utils/dater'

const props = defineProps<{
  database: Database
}>()

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
