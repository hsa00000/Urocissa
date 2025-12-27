<template>
  <v-list-item prepend-icon="mdi-star" @click="setFavorite(props.indexList, true, isolationId)">
    <v-list-item-title class="wrap">Favorite</v-list-item-title>
  </v-list-item>
</template>

<script lang="ts" setup>
import { useRoute } from 'vue-router'
import { getIsolationIdByRoute } from '@utils/getter'
import { setFavorite } from '@/api/editFlags'

const props = defineProps<{
  indexList: number[]
}>()

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)
</script>
