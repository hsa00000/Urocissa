<template>
  <v-list-item
    prepend-icon="mdi-restore"
    @click="quickRemoveTags('_trashed', props.indexList, isolationId)"
  >
    <v-list-item-title class="wrap">Restore</v-list-item-title>
  </v-list-item>
</template>
<script lang="ts" setup>
import { useRoute } from 'vue-router'
import { getIsolationIdByRoute } from '@utils/getter'
import { quickRemoveTags } from '@/script/common/quickEditTags'

const props = defineProps<{
  indexList: number[]
}>()

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)
</script>
