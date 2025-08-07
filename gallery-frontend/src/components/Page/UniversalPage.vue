<template>
  <HomeMain :basic-string="basicStringStore.basicString" />
</template>

<script setup lang="ts">
import HomeMain from '@/components/Home/HomeMain.vue'
import { useBasicStringStore } from '@/store/basicStringStore'
const basicStringStore = useBasicStringStore('mainId')
</script>
