<template>
  <v-dialog v-model="modalStore.showSettingModal" id="setting-modal" variant="flat" rounded>
    <v-card class="mx-auto w-100" max-width="400" variant="elevated" retain-focus>
      <v-card-title>Settings</v-card-title>
      <v-card-text class="pa-0">
        <v-table >
          <tbody>
            <tr>
              <td>
                <v-chip variant="text"> Thumbnail size </v-chip>
              </td>
              <td style="width: 250px;">
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
                  @click:prepend="onSubRowHeightScaleUpdate(-10)"
                  @click:append="onSubRowHeightScaleUpdate(10)"
                ></v-slider>
              </td>
            </tr>
            <tr>
              <td>
                <v-chip variant="text"> Show Filename Chip </v-chip>
              </td>
              <td style="width: 250px;">
                <v-switch
                  :model-value="showFilenameChipValue"
                  @update:model-value="onShowFilenameChipUpdate"
                  :disabled="!initializedStore.initialized"
                  hide-details
                ></v-switch>
              </td>
            </tr>
          </tbody>
        </v-table>
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

</script>
