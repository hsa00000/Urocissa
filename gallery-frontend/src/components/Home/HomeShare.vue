<script setup lang="ts">
import { useRoute } from 'vue-router'
import Home from './Home.vue'
import HomeShareBar from '@/components/NavBar/HomeBars/HomeShareBar.vue'
import ShareLoginModal from '@/components/Modal/ShareLoginModal.vue'
import { computed } from 'vue'
import { useShareRouteContext } from '@/script/hook/useShareRouteContext'

const route = useRoute()
const searchString = computed(() => route.query.search)
const { basicString, contextKey } = useShareRouteContext(route)
</script>

<template>
  <Home
    v-if="basicString !== undefined"
    :key="contextKey"
    isolation-id="mainId"
    :basic-string="basicString"
    :search-string="searchString"
  >
    <template #home-toolbar> <HomeShareBar /> </template>
  </Home>

  <ShareLoginModal />
</template>
