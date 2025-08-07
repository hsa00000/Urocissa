<template>
  <HomeMain :basic-string="basicString" />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import HomeMain from '@/components/Home/HomeMain.vue'

const route = useRoute()

// 根据 query 参数中的 type 来确定 basicString
const basicString = computed(() => {
  const type = route.query.type as string

  switch (type) {
    case 'home':
      return 'and(not(tag:"_archived"), not(tag:"_trashed"))'
    case 'all':
      return 'not(tag:"_trashed")'
    case 'favorite':
      return 'and(tag:"_favorite", not(tag:"_trashed"))'
    case 'archived':
      return 'and(tag:"_archived", not(tag:"_trashed"))'
    case 'trashed':
      return 'and(tag:"_trashed")'
    case 'albums':
      return 'and(type:"album", not(tag:"_trashed"))'
    case 'videos':
      return 'and(type:"video", not(tag:"_archived"), not(tag:"_trashed"))'
    default:
      return 'and(not(tag:"_archived"), not(tag:"_trashed"))'
  }
})
</script>
