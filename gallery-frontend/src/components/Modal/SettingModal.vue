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
              :model-value="subRowHeightScaleValue"
              @update:model-value="onSubRowHeightScaleUpdate"
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
        <v-row align="center" no-gutters class="mt-4">
          <v-col cols="auto">
            <v-chip variant="text"> Limit Ratio </v-chip>
          </v-col>
          <v-col>
            <v-switch
              :model-value="limitRatioValue"
              @update:model-value="onLimitRatioUpdate"
              :disabled="!initializedStore.initialized"
              hide-details
            ></v-switch>
          </v-col>
        </v-row>
        <v-row align="center" no-gutters class="mt-4">
          <v-col cols="auto">
            <v-chip variant="text"> Theme </v-chip>
          </v-col>
          <v-col>
            <v-switch
              :model-value="themeIsLight"
              @update:model-value="onThemeUpdate"
              :disabled="!initializedStore.initialized"
              hide-details
            ></v-switch>
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
import { computed } from 'vue'
import { useModalStore } from '@/store/modalStore'
import { useInitializedStore } from '@/store/initializedStore'
import { useConstStore } from '@/store/constStore'
import { useTheme } from 'vuetify'

const modalStore = useModalStore('mainId')
const initializedStore = useInitializedStore('mainId')
const constStore = useConstStore('mainId')
const vuetifyTheme = useTheme()

// Read-only computed for subRowHeightScale (source of truth is constStore)
const subRowHeightScaleValue = computed(() => constStore.subRowHeightScale)
// Read-only computed for limitRatio (source of truth is constStore)
const limitRatioValue = computed(() => constStore.limitRatio)

// computed boolean for light theme switch
const themeIsLight = computed(() => constStore.theme === 'light')

// Handler invoked when the slider updates its model value
const onSubRowHeightScaleUpdate = (newValue: number | null) => {
  const value = Number(newValue ?? constStore.subRowHeightScale)
  const clamped = Math.max(250, Math.min(450, value))
  constStore.updateSubRowHeightScale(clamped).catch((error: unknown) => {
    console.error('Failed to update subRowHeightScale:', error)
  })
}

// Handler invoked when the switch updates its model value
const onLimitRatioUpdate = (newValue: boolean | null) => {
  const value = !!newValue
  constStore.updateLimitRation(value).catch((error: unknown) => {
    console.error('Failed to update limitRatio:', error)
  })
}

// Handler for theme switch
const onThemeUpdate = async (newValue: boolean | null) => {
  const wantLight = !!newValue
  const newTheme = wantLight ? 'light' : 'dark'
  try {
    await constStore.updateTheme(newTheme)
    // update Vuetify theme instance
    if (vuetifyTheme && typeof vuetifyTheme.change === 'function') {
      vuetifyTheme.change(newTheme)
    }
  } catch (err) {
    console.error('Failed to update theme:', err)
  }
}

// Function to adjust thumbnail size with icons
const adjustThumbnailSize = (delta: number) => {
  const currentValue = constStore.subRowHeightScale
  const newValue = Math.max(250, Math.min(450, currentValue + delta))

  // Only update if the value would actually change
  if (newValue !== currentValue) {
    constStore.updateSubRowHeightScale(newValue).catch((error: unknown) => {
      console.error('Failed to update subRowHeightScale via adjustThumbnailSize:', error)
    })
  }
}
</script>
