<template>
  <v-list-item prepend-icon="mdi-restore" @click="setTrashed(props.indexList, false, isolationId)">
    <v-list-item-title class="wrap">Restore</v-list-item-title>
  </v-list-item>
</template>
<script lang="ts" setup>
import { useRoute } from 'vue-router'
import { getIsolationIdByRoute } from '@utils/getter'
import { setTrashed } from '@/api/editFlags'

const props = defineProps<{
  indexList: number[]
}>()

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)
</script>
