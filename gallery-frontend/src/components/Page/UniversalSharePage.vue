<template>
  <HomeMain :filter-string="filterStringStore.filterString" />
</template>

<script setup lang="ts">
import HomeMain from '@/components/Home/HomeMain.vue'
import { useFilterStringStore } from '@/store/filterStringStore'

const filterStringStore = useFilterStringStore()
</script>
