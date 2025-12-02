<template>
  <v-menu location="start">
    <template #activator="{ props: MenuBtn }">
      <v-btn v-bind="MenuBtn" icon="mdi-dots-vertical"></v-btn>
    </template>
    <v-list>
      <ItemViewOriginalFile
        v-if="shareStore.resolvedShare === null || shareStore.resolvedShare.share.showDownload"
        :src="getSrcOriginal(database.hash, true, database.ext, tokenStore.hashTokenMap.get(database.hash))"
        :hash="database.hash"
        :isolation-id="props.isolationId"
      />
      <ItemDownload
        v-if="shareStore.resolvedShare === null || shareStore.resolvedShare.share.showDownload"
        :index-list="[props.index]"
      />
    </v-list>
  </v-menu>
</template>
<script setup lang="ts">
import { Database, IsolationId } from '@type/types'
import { getSrcOriginal } from '@utils/getter'
import { useTokenStore } from '@/store/tokenStore'
import { useShareStore } from '@/store/shareStore'
import ItemViewOriginalFile from '@Menu/MenuItem/ItemViewOriginalFile.vue'
import ItemDownload from '@Menu/MenuItem/ItemDownload.vue'
const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  database: Database
}>()
const tokenStore = useTokenStore(props.isolationId)
const shareStore = useShareStore('mainId')
</script>
