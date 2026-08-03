<template>
  <div class="pa-2">
    <v-textarea
      v-model="model.description"
      label="Link Description"
      placeholder="Add a description"
      variant="outlined"
      density="compact"
      bg-color="#2a2a2a"
      color="primary"
      rows="3"
      auto-grow
      hide-details="auto"
      class="mb-4 rounded-lg"
    ></v-textarea>

    <v-text-field
      v-model="model.password"
      label="Password"
      placeholder="Enter password"
      hint="Require a password to access this link"
      persistent-hint
      type="password"
      variant="outlined"
      density="compact"
      bg-color="#2a2a2a"
      color="primary"
      hide-details="auto"
      single-line
      clearable
      prepend-inner-icon="mdi-lock-outline"
      class="mb-4 rounded-lg"
    ></v-text-field>

    <v-select
      v-model="model.expDuration"
      :items="DURATIONS"
      item-title="label"
      item-value="id"
      label="Expiration"
      placeholder="Never"
      hint="Set when this link should expire"
      persistent-hint
      variant="outlined"
      density="compact"
      bg-color="#2a2a2a"
      color="primary"
      hide-details="auto"
      single-line
      clearable
      prepend-inner-icon="mdi-clock-outline"
      class="mb-4 rounded-lg"
      @click:clear="model.expDuration = null"
    ></v-select>

    <v-divider class="mx-n6 my-4 border-opacity-25"></v-divider>

    <v-list lines="one" class="bg-transparent pa-0">
      <v-list-item
        v-for="item in settingsItems"
        :key="item.key"
        :title="item.title"
        class="mb-1 rounded-lg"
        @click="toggleSetting(item.key)"
        link
      >
        <template #append>
          <v-switch
            :model-value="model[item.key]"
            color="primary"
            hide-details
            density="compact"
            inset
            readonly
            class="pointer-events-none ml-2"
          ></v-switch>
        </template>
      </v-list-item>
    </v-list>
  </div>
</template>

<script setup lang="ts">
import { watch } from 'vue'
import { DURATIONS } from '@type/constants'
import { ShareFormData } from '@type/types'

const model = defineModel<ShareFormData>({ required: true })

const settingsItems = [

  { title: 'Show Metadata', key: 'showMetadata' },
  { title: 'Allow Download', key: 'showDownload' },
  { title: 'Allow Upload', key: 'showUpload' }
] as const

const toggleSetting = (
  key: keyof Pick<ShareFormData, 'showMetadata' | 'showDownload' | 'showUpload'>
) => {
  model.value[key] = !model.value[key]
}

// --- Logic Automation ---

// 1. Password Logic
watch(
  () => model.value.password,
  (newVal) => {
    model.value.passwordRequired = !!(newVal && newVal.length > 0)
  }
)

// 2. Expiration Logic
watch(
  () => model.value.expDuration,
  (newVal) => {
    // eslint-disable-next-line @typescript-eslint/strict-boolean-expressions
    model.value.expireEnabled = !!newVal
  }
)

</script>

<style scoped>
/* Utility class to ensure the v-switch is purely visual 
  and the interaction is handled by the parent v-list-item.
*/
.pointer-events-none {
  pointer-events: none;
}
</style>
