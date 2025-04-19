<template>
  <v-menu location="start">
    <template #activator="{ props: MenuBtn }">
      <v-btn v-bind="MenuBtn" icon="mdi-dots-vertical"></v-btn>
    </template>
    <v-list>
      <ItemViewOriginalFile
        :src="getSrc(database.hash, true, database.ext, Cookies.get('jwt')!, undefined)"
      />
      <ItemDownload :index-list="[props.index]" />
    </v-list>
  </v-menu>
</template>
<script setup lang="ts">
import { Database, IsolationId } from '@type/types'
import { getSrc } from '@/../config'
import Cookies from 'js-cookie'
import ItemViewOriginalFile from '@Menu/MenuItem/ItemViewOriginalFile.vue'
import ItemDownload from '@Menu/MenuItem/ItemDownload.vue'
const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  database: Database
}>()
</script>
