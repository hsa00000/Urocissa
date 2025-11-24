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
import { Database } from '@type/types'
import { computed } from 'vue'


const props = defineProps<{
  database: Database
}>()

const filePathComplete = computed(() => {
  return props.database.filename
})

const filePath = computed(() => {
  if (filePathComplete.value != null) {
    return filePathComplete.value.replace(/\.[^/.]+$/, '')
  }
  return ''
})
</script>
