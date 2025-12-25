<template>
  <div
    id="abstractData-col"
    v-if="abstractData"
    class="h-100 flex-grow-0 flex-shrink-0 bg-surface"
    :style="{
      width: constStore.showInfo ? '360px' : '0',
      zIndex: 1
    }"
  >
    <div class="position-relative">
      <div>
        <v-toolbar class="bg-surface">
          <v-btn icon @click="toggleInfo">
            <v-icon>mdi-close</v-icon>
          </v-btn>
          <v-toolbar-title class="text-h5">Info</v-toolbar-title>
        </v-toolbar>
      </div>
      <v-card-item v-if="!isShareMode || userDefinedDescriptionModel">
        <v-textarea
          v-model="userDefinedDescriptionModel"
          :variant="isShareMode ? 'plain' : 'underlined'"
          :readonly="isShareMode"
          rows="1"
          auto-grow
          @blur="
            !isShareMode &&
              editUserDefinedDescription(
                props.abstractData,
                userDefinedDescriptionModel,
                props.index,
                props.isolationId
              )
          "
          :placeholder="
            !isShareMode && userDefinedDescriptionModel === '' ? 'Add description' : undefined
          "
        />
      </v-card-item>
      <div
        v-if="abstractData.type === 'image' || abstractData.type === 'video'"
        class="h-100 w-100"
      >
        <v-list class="pa-0" height="100%" lines="two">
          <ItemSize :database="abstractData" />
          <ItemPath v-if="showMetadata" :database="abstractData" />
          <ItemDate :database="abstractData" />
          <ItemExif
            v-if="abstractData.exif.Make !== undefined || abstractData.exif.Model !== undefined"
            :database="abstractData"
          />
          <v-divider></v-divider>
          <ItemTag
            v-if="showMetadata"
            :isolation-id="props.isolationId"
            :index="props.index"
            :tags="abstractData.tags"
            :is-favorite="abstractData.isFavorite"
            :is-archived="abstractData.isArchived"
          />
          <ItemAlbum
            v-if="route.meta.baseName !== 'share'"
            :isolation-id="props.isolationId"
            :index="props.index"
            :albums="abstractData.albums"
          />
        </v-list>
      </div>
      <div v-if="abstractData.type === 'album'" class="h-100 w-100">
        <v-list class="pa-0" height="100%" lines="two">
          <ItemTitle :title="abstractData.title" />
          <ItemCount :album="abstractData" />
          <v-divider></v-divider>
          <ItemTag
            :isolation-id="props.isolationId"
            :index="props.index"
            :tags="abstractData.tags"
            :is-favorite="abstractData.isFavorite"
            :is-archived="abstractData.isArchived"
          />
        </v-list>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, watch, ref } from 'vue'
import { useConstStore } from '@/store/constStore'

import { EnrichedUnifiedData, IsolationId } from '@type/types'

import ItemExif from './ItemExif.vue'
import ItemSize from './ItemSize.vue'
import ItemPath from './ItemPath.vue'
import ItemDate from './ItemDate.vue'
import ItemTag from './ItemTag.vue'
import ItemAlbum from './ItemAlbum.vue'
import ItemTitle from './ItemTitle.vue'
import ItemCount from './ItemCount.vue'
import { useRoute } from 'vue-router'
import { useShareStore } from '@/store/shareStore'
import { editUserDefinedDescription } from '@utils/editDescription'

const route = useRoute()

const userDefinedDescriptionModel = ref('')

const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  abstractData: EnrichedUnifiedData
}>()

const showMetadata = computed(() => {
  return route.meta.baseName !== 'share' || shareStore.resolvedShare?.share.showMetadata
})

// 新增：判斷是否為分享模式
const isShareMode = computed(() => {
  return route.meta.baseName === 'share'
})

const constStore = useConstStore('mainId')
const shareStore = useShareStore('mainId')

function toggleInfo() {
  void constStore.updateShowInfo(!constStore.showInfo)
}

function getUserDefinedDescription(abstractData: EnrichedUnifiedData): string {
  return abstractData.description ?? ''
}

watch(
  () => props.hash,
  () => {
    userDefinedDescriptionModel.value = getUserDefinedDescription(props.abstractData)
  },
  { immediate: true }
)
</script>

<style scoped>
@media (width <= 720px) {
  /* On small screens, make the info pane full width.
     Use !important to override the inline :style binding for width. */
  #abstractData-col {
    width: 100% !important;
  }
}
</style>
