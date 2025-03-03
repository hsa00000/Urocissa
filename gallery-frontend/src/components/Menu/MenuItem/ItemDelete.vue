<template>
  <v-list-item
    prepend-icon="mdi-trash-can-outline"
    @click="quickAddTags('_trashed', props.indexList, isolationId)"
  >
    <v-list-item-title class="wrap">Delete</v-list-item-title>
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
