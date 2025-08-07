<template>
  <HomeMain :basic-string="basicString" />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import HomeMain from '@/components/Home/HomeMain.vue'
import { useBasicStringStore } from '@/store/basicStringStore'

const basicStringStore = useBasicStringStore('mainId')

// 从 store 获取 basicString
const basicString = computed(() => {
  return basicStringStore.basicString
})
</script>
