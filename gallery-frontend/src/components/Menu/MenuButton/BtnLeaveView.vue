<template>
  <v-btn icon="mdi mdi-arrow-left" @click="leave(router)"></v-btn>
</template>
<script setup lang="ts">
import { useRouter } from 'vue-router'

import { leave } from '@/script/navigator'

const router = useRouter()
</script>
