<template>
  <div class="metadata-content h-100 w-100 bg-surface">
    <div class="metadata-content__scroll position-relative h-100">
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
          data-testid="edit-description"
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
      <div v-if="abstractData.type === 'image' || abstractData.type === 'video'" class="w-100">
        <v-list
          class="metadata-list pa-0"
          lines="two"
          :density="metadataItemDensity"
          :slim="metadataItemSlim"
        >
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
      <div v-if="abstractData.type === 'album'" class="w-100">
        <v-list
          class="metadata-list pa-0"
          lines="two"
          :density="metadataItemDensity"
          :slim="metadataItemSlim"
        >
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
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useConstStore } from '@/store/constStore'
import { useShareStore } from '@/store/shareStore'
import { editUserDefinedDescription } from '@utils/editDescription'
import { EnrichedUnifiedData, IsolationId } from '@type/types'
import ItemExif from './ItemExif.vue'
import ItemSize from './ItemSize.vue'
import ItemPath from './ItemPath.vue'
import ItemDate from './ItemDate.vue'
import ItemTag from './ItemTag.vue'
import ItemAlbum from './ItemAlbum.vue'
import ItemTitle from './ItemTitle.vue'
import ItemCount from './ItemCount.vue'
import { provideMetadataItemLayout } from './useMetadataItemLayout'

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

const isShareMode = computed(() => {
  return route.meta.baseName === 'share'
})

const constStore = useConstStore('mainId')
const shareStore = useShareStore('mainId')
const { metadataItemDensity, metadataItemSlim } = provideMetadataItemLayout()

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
.metadata-content {
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.metadata-content__scroll {
  min-width: 0;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
}

.metadata-list {
  overflow: visible;
}

:deep(.v-list-item-subtitle) {
  overflow: visible;
}
</style>
