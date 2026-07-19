<template>
  <v-dialog v-model="modalStore.showSettingModal" id="setting-modal" max-width="500">
    <v-card border flat class="rounded-lg">
      <v-card-title class="font-weight-bold">Settings</v-card-title>
      <v-divider thickness="4" variant="double"></v-divider>

      <v-list-item class="pt-4">
        <v-list-item-title class="mb-2">Thumbnail size</v-list-item-title>
        <v-slider
          show-ticks="always"
          v-model="subRowHeightScaleValue"
          :min="250"
          :max="450"
          :step="10"
          :disabled="!initializedStore.initialized"
          hide-details
          thumb-size="16"
          prepend-icon="mdi-minus"
          append-icon="mdi-plus"
          color="primary"
          @click:prepend="onSubRowHeightScaleUpdate(-10)"
          @click:append="onSubRowHeightScaleUpdate(10)"
        ></v-slider>
      </v-list-item>

      <v-divider></v-divider>

      <v-list-item
        title="Show Filename Chip"
        @click="onShowFilenameChipUpdate(!showFilenameChipValue)"
      >
        <template #append>
          <v-switch
            :model-value="showFilenameChipValue"
            @update:model-value="onShowFilenameChipUpdate"
            :disabled="!initializedStore.initialized"
            color="primary"
            inset
            hide-details
            @click.stop
          ></v-switch>
        </template>
      </v-list-item>

      <v-divider></v-divider>

      <v-list-item @click="onLimitRationUpdate(!limitRatioValue)">
        <template #title>
          <div class="d-flex align-center">
            Restricted Ratio
            <v-tooltip location="top" max-width="300">
              <template #activator="{ props }">
                <v-icon
                  v-bind="props"
                  icon="mdi-alert-circle-outline"
                  size="small"
                  color="medium-emphasis"
                  class="ml-2"
                  style="cursor: help"
                ></v-icon>
              </template>
              <span>
                <b>For extreme aspect-ratio images:</b><br />
                <b>On:</b> Containers stay between 1:2 and 2:1, cropping the excess.<br />
                <b>Off:</b> Images use their original aspect ratio.
              </span>
            </v-tooltip>
          </div>
        </template>
        <template #append>
          <v-switch
            :model-value="limitRatioValue"
            @update:model-value="onLimitRationUpdate"
            :disabled="!initializedStore.initialized"
            color="primary"
            inset
            hide-details
            @click.stop
          ></v-switch>
        </template>
      </v-list-item>

      <v-divider></v-divider>

      <v-list-item @click="onViewBarOverlayUpdate(!viewBarOverlayValue)">
        <template #title>
          <div class="d-flex align-center">
            ViewBar Overlay
            <v-tooltip location="top" max-width="300">
              <template #activator="{ props }">
                <v-icon
                  v-bind="props"
                  icon="mdi-alert-circle-outline"
                  size="small"
                  color="medium-emphasis"
                  class="ml-2"
                  style="cursor: help"
                ></v-icon>
              </template>
              <span>
                <b>When viewing photos:</b><br />
                <b>On:</b> The navigation bar overlays the image.<br />
                <b>Off:</b> The image is pushed down to prevent obstruction.
              </span>
            </v-tooltip>
          </div>
        </template>
        <template #append>
          <v-switch
            :model-value="viewBarOverlayValue"
            @update:model-value="onViewBarOverlayUpdate"
            :disabled="!initializedStore.initialized"
            color="primary"
            inset
            hide-details
            @click.stop
          ></v-switch>
        </template>
      </v-list-item>

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

const modalStore = useModalStore('mainId')
const initializedStore = useInitializedStore('mainId')
const constStore = useConstStore('mainId')

// Read/write computed for subRowHeightScale (source of truth is constStore)
const subRowHeightScaleValue = computed<number>({
  get: () => constStore.subRowHeightScale,
  set: (newVal: number | null) => {
    const value = newVal ?? constStore.subRowHeightScale
    const clamped = Math.max(250, Math.min(450, value))
    constStore.updateSubRowHeightScale(clamped).catch((error: unknown) => {
      console.error('Failed to update subRowHeightScale (via setter):', error)
    })
  }
})

// Read/write computed for showFilenameChip (source of truth is constStore)
const showFilenameChipValue = computed<boolean>({
  get: () => constStore.showFilenameChip,
  set: (newVal: boolean | null) => {
    constStore.updateShowFilenameChip(newVal ?? true).catch((error: unknown) => {
      console.error('Failed to update showFilenameChip (via setter):', error)
    })
  }
})

// Handler invoked when the slider updates its model value
const onSubRowHeightScaleUpdate = (newValue: number | null) => {
  const value = newValue ?? constStore.subRowHeightScale
  const clamped = Math.max(250, Math.min(450, value))
  constStore.updateSubRowHeightScale(clamped).catch((error: unknown) => {
    console.error('Failed to update subRowHeightScale:', error)
  })
}

const onShowFilenameChipUpdate = (newValue: boolean | null) => {
  constStore.updateShowFilenameChip(newValue ?? true).catch((error: unknown) => {
    console.error('Failed to update showFilenameChip:', error)
  })
}

const limitRatioValue = computed(() => constStore.limitRatio)

const onLimitRationUpdate = (newValue: boolean | null) => {
  constStore.updateLimitRation(newValue ?? false).catch((error: unknown) => {
    console.error('Failed to update limitRatio:', error)
  })
}

const viewBarOverlayValue = computed<boolean>({
  get: () => constStore.viewBarOverlay,
  set: (newVal: boolean | null) => {
    if (newVal !== null) {
      constStore.updateViewBarOverlay(newVal).catch((error: unknown) => {
        console.error('Failed to update viewBarOverlay (via setter):', error)
      })
    }
  }
})

const onViewBarOverlayUpdate = (newValue: boolean | null) => {
  if (newValue !== null) {
    constStore.updateViewBarOverlay(newValue).catch((error: unknown) => {
      console.error('Failed to update viewBarOverlay:', error)
    })
  }
}
</script>
