<template>
  <v-btn icon="mdi-book-plus" :loading="loading" @click="createEmptyAlbumWithLoading()" />
</template>

<script setup lang="ts">
import { ref, watchEffect } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { createEmptyAlbum } from '@utils/createAlbums'
import { navigateToAlbum } from '@/route/navigator'

const loading = defineModel<boolean>()

const route = useRoute()
const router = useRouter()
const searchQuery = ref('')

watchEffect(() => {
  searchQuery.value = route.query.search as string
})

const createEmptyAlbumWithLoading = async () => {
  loading.value = true
  const newAlbumId = await createEmptyAlbum()
  if (typeof newAlbumId === 'string') {
    await navigateToAlbum(newAlbumId, router)
  }
  loading.value = false
}
</script>
