<template>
  <v-dialog v-model="modalStore.showSettingModal" id="setting-modal" variant="flat" rounded>
    <v-card class="mx-auto w-100" max-width="400" variant="elevated" retain-focus>
      <v-card-title>Settings</v-card-title>
      <v-card-text>
        <v-row align="center" no-gutters>
          <v-col cols="auto">
            <v-chip variant="text"> Thumbnail size </v-chip>
          </v-col>
          <v-col>
            <v-slider
              show-ticks="always"
              v-model="subRowHeightScale"
              :min="250"
              :max="450"
              :step="10"
              :thumb-label="true"
              :disabled="!initializedStore.initialized"
              hide-details
              thumb-size="16"
              prepend-icon="mdi-minus"
              append-icon="mdi-plus"
              @click:prepend="adjustThumbnailSize(-10)"
              @click:append="adjustThumbnailSize(10)"
            ></v-slider>
          </v-col>
        </v-row>
      </v-card-text>
      <v-card-actions>
        <v-spacer></v-spacer>
        <v-btn @click="modalStore.showSettingModal = false">Close</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { watchDebounced } from '@vueuse/core'
import { useModalStore } from '@/store/modalStore'
import { useInitializedStore } from '@/store/initializedStore'
import { useConstStore } from '@/store/constStore'

const modalStore = useModalStore('mainId')
const initializedStore = useInitializedStore('mainId')
const constStore = useConstStore()

// Local ref for immediate UI updates
const subRowHeightScale = ref(constStore.subRowHeightScale)

// Watch store changes to sync local ref
watch(
  () => constStore.subRowHeightScale,
  (newValue) => {
    subRowHeightScale.value = newValue
  }
)

watchDebounced(
  subRowHeightScale,
  (newValue: number) => {
    constStore.updateSubRowHeightScale(newValue).catch((error: unknown) => {
      console.error('Failed to update subRowHeightScale:', error)
    })
  },
  { debounce: 50 }
)

// Function to adjust thumbnail size with icons
const adjustThumbnailSize = (delta: number) => {
  const currentValue = subRowHeightScale.value
  const newValue = Math.max(250, Math.min(450, currentValue + delta))

  // Only update if the value would actually change
  if (newValue !== currentValue) {
    subRowHeightScale.value = newValue
  }
}
</script>
