<template>
  <v-list-item
    prepend-icon="mdi-archive-arrow-down"
    @click="quickAddTags('_archived', props.indexList, isolationId)"
  >
    <v-list-item-title class="wrap">Archive</v-list-item-title>
  </v-list-item>
</template>

<script lang="ts" setup>
import { useRoute } from 'vue-router'
import { getIsolationIdByRoute } from '@utils/getter'
import { quickAddTags } from '@/script/common/quickEditTags'

const props = defineProps<{
  indexList: number[]
}>()

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)
</script>
