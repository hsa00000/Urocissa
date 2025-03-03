<template>
  <v-list-item
    prepend-icon="mdi-book-plus"
    value="create-album"
    @click="createEmptyAlbumWithLoading()"
  >
    <v-list-item-title class="wrap">{{ 'Create Album' }}</v-list-item-title>
  </v-list-item>
</template>

<script setup lang="ts">
import { ref, watchEffect } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { createEmptyAlbum } from '@/script/common/createAlbums'
import { getIsolationIdByRoute } from '@utils/getter'

const loading = defineModel<boolean>()

const route = useRoute()
const router = useRouter()
const searchQuery = ref('')

const isolationId = getIsolationIdByRoute(route)

watchEffect(() => {
  searchQuery.value = route.query.search as string
})

const createEmptyAlbumWithLoading = async () => {
  loading.value = true
  await createEmptyAlbum(isolationId, router)
  loading.value = false
}
</script>
