<template>
  <PageTemplate>
    <template #content>
      <HomeMain :basic-string="basicString" />
    </template>
  </PageTemplate>
</template>

<script setup lang="ts">
import HomeMain from '@/components/Home/HomeMain.vue'
import PageTemplate from './PageLayout/PageTemplate.vue'
const basicString = 'trashed:true'
</script>
