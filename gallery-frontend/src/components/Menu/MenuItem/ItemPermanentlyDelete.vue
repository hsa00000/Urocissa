<template>
  <v-list-item prepend-icon="mdi-trash-can-outline" @click="deleteData">
    <v-list-item-title class="wrap">Permanently Delete</v-list-item-title>
  </v-list-item>
</template>

<script lang="ts" setup>
import { useRoute } from 'vue-router'
import { deleteDataInWorker } from '@/script/inWorker/deleteDataInWorker'
import { getIsolationIdByRoute } from '@utils/getter'
const route = useRoute()
const isolationId = getIsolationIdByRoute(route)

const props = defineProps<{
  indexList: number[]
}>()

const deleteData = () => {
  deleteDataInWorker(props.indexList, isolationId)
}
</script>
