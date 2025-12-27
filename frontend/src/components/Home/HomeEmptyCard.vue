<template>
  <v-container class="d-flex align-center justify-center" fluid>
    <!-- No search results -->
    <v-row v-if="ui.isSearchEmpty" justify="center">
      <v-col class="w-100" cols="12" md="6" lg="4">
        <v-hover v-slot="{ isHovering, props: hoverProps }">
          <v-card
            class="pa-4 text-center mx-auto"
            :class="{ 'hover-cursor': ui.hasHoverEffect }"
            :style="{
              border:
                ui.hasHoverEffect && isHovering ? '2px solid #BDBDBD' : '2px solid transparent'
            }"
            :elevation="ui.hasHoverEffect && isHovering ? 12 : 2"
            rounded="lg"
            width="100%"
            v-bind="ui.hasHoverEffect ? hoverProps : {}"
          >
            <v-icon class="mb-5" color="grey" size="100"> {{ ui.icon }} </v-icon>
            <v-card-item>
              <v-card-subtitle>{{ ui.message }}</v-card-subtitle>
            </v-card-item>
          </v-card>
        </v-hover>
      </v-col>
    </v-row>

    <!-- General empty state -->
    <v-row v-else justify="center">
      <!-- Left: Upload-only card, shown only when level===3 and not in edit mode -->
      <v-col
        v-if="ui.showUploadCard && typeof route.params.hash === 'string'"
        class="w-100"
        cols="12"
        md="6"
        lg="4"
      >
        <v-hover v-slot="{ isHovering, props: hoverProps }">
          <v-card
            class="pa-4 text-center mx-auto"
            :class="{ 'hover-cursor': true }"
            :style="{ border: isHovering ? '2px solid #BDBDBD' : '2px solid transparent' }"
            :elevation="isHovering ? 12 : 2"
            rounded="lg"
            width="100%"
            v-bind="hoverProps"
            @click="uploadStore.triggerFileInput(route.params.hash)"
          >
            <v-icon class="mb-5" color="grey" size="100">mdi-cloud-upload</v-icon>
            <v-card-item>
              <v-card-subtitle>Upload new photos.</v-card-subtitle>
            </v-card-item>
          </v-card>
        </v-hover>
      </v-col>

      <!-- Right: Main message card (click, hover, and message are all determined by the same computed property) -->
      <v-col class="w-100" cols="12" md="6" lg="4">
        <v-hover v-slot="{ isHovering, props: hoverProps }">
          <v-card
            class="pa-4 text-center mx-auto"
            :class="{ 'hover-cursor': ui.hasHoverEffect }"
            :style="{
              border:
                ui.hasHoverEffect && isHovering ? '2px solid #BDBDBD' : '2px solid transparent'
            }"
            :elevation="ui.hasHoverEffect && isHovering ? 12 : 2"
            rounded="lg"
            width="100%"
            v-bind="ui.hasHoverEffect ? hoverProps : {}"
            @click="ui.onClick ? ui.onClick() : undefined"
          >
            <v-icon class="mb-5" color="grey" size="100"> {{ ui.icon }} </v-icon>
            <v-card-item>
              <v-card-subtitle>{{ ui.message }}</v-card-subtitle>
            </v-card-item>
          </v-card>
        </v-hover>
      </v-col>
    </v-row>
  </v-container>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useCollectionStore } from '@/store/collectionStore'
import { useModalStore } from '@/store/modalStore'
import { useUploadStore } from '@/store/uploadStore'
import { createEmptyAlbum } from '@utils/createAlbums'
import { navigateToAlbum } from '@/route/navigator'
import type { IsolationId } from '@type/types'

const props = defineProps<{
  isolationId: IsolationId
}>()

const route = useRoute()
const router = useRouter()

const uploadStore = useUploadStore('mainId')
const collectionStore = useCollectionStore(props.isolationId)
const modalStore = useModalStore('mainId')

type ClickHandler = (() => void | Promise<void>) | undefined

interface UIState {
  isSearchEmpty: boolean
  showUploadCard: boolean
  hasHoverEffect: boolean
  message: string
  icon: string
  onClick: ClickHandler
}

/**
 * Single computed property with a single switch (plus a few preconditions),
 * determines at once: whether to show two cards, hover, message, icon, and click behavior.
 */
const ui = computed<UIState>(() => {
  const isSearching =
    route.query.search !== undefined && route.query.search?.toString().trim() !== ''

  // 1) No search results: always show a single card, no click
  if (isSearching) {
    return {
      isSearchEmpty: true,
      showUploadCard: false,
      hasHoverEffect: false,
      message: 'Result not found.',
      icon: 'mdi-book-remove-multiple',
      onClick: undefined
    }
  }

  // 2) Inside album (level === 3)
  if (route.meta.level === 3) {
    // adding photos mode: only main card, no click
    if (collectionStore.editModeOn) {
      return {
        isSearchEmpty: false,
        showUploadCard: false,
        hasHoverEffect: false,
        message: 'All photos are already added!',
        icon: 'mdi-image-plus',
        onClick: undefined
      }
    }
    // empty album mode: two cards (left = upload; right = select from existing photos)
    return {
      isSearchEmpty: false,
      showUploadCard: true,
      hasHoverEffect: true,
      message: 'Select from existing photos.',
      icon: 'mdi-image-plus',
      onClick: () => {
        modalStore.showHomeTempModal = true
      }
    }
  }

  // 3) Other pages: single switch based on baseName
  switch (route.meta.baseName) {
    case 'home':
    case 'all':
    case 'album':
      return {
        isSearchEmpty: false,
        showUploadCard: false,
        hasHoverEffect: true,
        message: 'Upload some photos here!',
        icon: 'mdi-image-plus',
        onClick: () => {
          uploadStore.triggerFileInput(undefined)
        }
      }

    case 'albums':
      return {
        isSearchEmpty: false,
        showUploadCard: false,
        hasHoverEffect: true,
        message: 'Create some albums here!',
        icon: 'mdi-image-album',
        onClick: async () => {
          const newAlbumId = await createEmptyAlbum()
          if (typeof newAlbumId === 'string') {
            await navigateToAlbum(newAlbumId, router)
          }
        }
      }

    case 'favorite':
      return {
        isSearchEmpty: false,
        showUploadCard: false,
        hasHoverEffect: false,
        message: 'Add your favorite photos and videos here!',
        icon: 'mdi-star',
        onClick: undefined
      }

    case 'archived':
      return {
        isSearchEmpty: false,
        showUploadCard: false,
        hasHoverEffect: false,
        message: 'Archived photos won’t appear on the home page.',
        icon: 'mdi-archive-arrow-down',
        onClick: undefined
      }

    case 'trashed':
      return {
        isSearchEmpty: false,
        showUploadCard: false,
        hasHoverEffect: false,
        message: 'Deleted photos and videos appear here.',
        icon: 'mdi-delete-outline',
        onClick: undefined
      }

    case 'videos':
      return {
        isSearchEmpty: false,
        showUploadCard: false,
        hasHoverEffect: false,
        message: 'Upload some videos here!',
        icon: 'mdi-play-circle-outline',
        onClick: undefined
      }

    case 'tags':
      return {
        isSearchEmpty: false,
        showUploadCard: false,
        hasHoverEffect: false,
        message: 'Organize your photos with tags!',
        icon: 'mdi-tag-outline',
        onClick: undefined
      }

    case 'login':
      return {
        isSearchEmpty: false,
        showUploadCard: false,
        hasHoverEffect: false,
        message: 'Sign in to access your photos.',
        icon: 'mdi-login',
        onClick: undefined
      }

    default:
      return {
        isSearchEmpty: false,
        showUploadCard: false,
        hasHoverEffect: false,
        message: 'Upload some photos here!',
        icon: 'mdi-image-plus',
        onClick: undefined
      }
  }
})
</script>

<style scoped>
.hover-cursor {
  cursor: pointer !important;
}
</style>
