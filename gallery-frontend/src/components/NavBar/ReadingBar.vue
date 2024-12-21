<template>
  <v-toolbar
    flat
    height="2"
    class="no-select"
    :style="{
      backgroundColor: '#212121'
    }"
  >
  </v-toolbar>
  <v-toolbar
    v-if="!collectionStore.editModeOn"
    class="position-relative"
    :style="{
      backgroundColor: '#212121'
    }"
  >
    <v-btn icon="mdi mdi-arrow-left" :to="leaveRead(route)"></v-btn
    ><v-card elevation="0">
      <v-card-title> {{ props.title }}</v-card-title>
    </v-card>
    <v-spacer></v-spacer>
    <v-btn icon="mdi-plus" @click="showHomeTemp = !showHomeTemp"> </v-btn>
  </v-toolbar>
  <EditBar v-else />
  <ProgessBar />
  <HomeTemp v-if="showHomeTemp" />
</template>
<script setup lang="ts">
import { useCollectionStore } from '@/store/collectionStore'
import { leaveRead } from '@/script/navigator'
import { useRoute } from 'vue-router'
import EditBar from './EditBar.vue'
import ProgessBar from './ProgessBar.vue'
import { ref } from 'vue'
import HomeTemp from '../Home/HomeTemp.vue'
const showHomeTemp = ref(false)

const props = defineProps<{
  title: string | null
}>()
const route = useRoute()
const collectionStore = useCollectionStore('subId')
</script>
