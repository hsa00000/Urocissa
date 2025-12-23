<template>
  <div>
    <v-textarea
      v-model="formData.description"
      label="Link Description"
      variant="outlined"
      density="compact"
      rows="1"
      auto-grow
      hide-details
      class="mb-4"
      color="primary"
      bg-color="grey-darken-4"
    ></v-textarea>

    <div class="text-caption text-medium-emphasis mb-2 text-uppercase font-weight-bold">
      Permissions
    </div>
    <v-row dense class="mb-2">
      <v-col cols="6">
        <v-switch
          v-model="formData.showDownload"
          label="Allow Download"
          color="primary"
          density="compact"
          hide-details
          inset
        ></v-switch>
      </v-col>
      <v-col cols="6">
        <v-switch
          v-model="formData.showUpload"
          label="Allow Upload"
          color="primary"
          density="compact"
          hide-details
          inset
        ></v-switch>
      </v-col>
      <v-col cols="12">
        <v-switch
          v-model="formData.showMetadata"
          label="Show Metadata"
          color="primary"
          density="compact"
          hide-details
          inset
        ></v-switch>
      </v-col>
    </v-row>

    <v-divider class="mb-4 border-opacity-25"></v-divider>

    <v-row dense align="center" class="mb-1">
      <v-col cols="5">
        <v-switch
          v-model="formData.passwordRequired"
          label="Password"
          color="primary"
          density="compact"
          hide-details
          inset
        ></v-switch>
      </v-col>
      <v-col cols="7">
        <v-text-field
          ref="passwordInputRef"
          v-model="formData.password"
          :disabled="!formData.passwordRequired"
          type="password"
          hide-details
          density="compact"
          variant="outlined"
          bg-color="grey-darken-4"
          prepend-inner-icon="mdi-lock-outline"
        ></v-text-field>
      </v-col>
    </v-row>

    <v-row dense align="center">
      <v-col cols="5">
        <v-switch
          v-model="toggleExpiration"
          label="Expiration"
          color="primary"
          density="compact"
          hide-details
          inset
        ></v-switch>
      </v-col>
      <v-col cols="7">
        <v-select
          v-model="formData.expDuration"
          :items="DURATIONS"
          :disabled="!formData.expireEnabled"
          :label="durationLabel"
          density="compact"
          variant="outlined"
          hide-details
          bg-color="grey-darken-4"
          prepend-inner-icon="mdi-clock-outline"
          item-title="label"
          item-value="id"
        ></v-select>
      </v-col>
    </v-row>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, computed, PropType } from 'vue'
import { DURATIONS } from '@type/constants'

export interface ShareFormData {
  description: string
  passwordRequired: boolean
  password: string
  expireEnabled: boolean
  expDuration: number | null
  showUpload: boolean
  showDownload: boolean
  showMetadata: boolean
}

const props = defineProps({
  modelValue: {
    type: Object as PropType<ShareFormData>,
    required: true
  },
  durationLabel: {
    type: String,
    default: 'Duration'
  }
})

const emit = defineEmits(['update:modelValue'])

const formData = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val)
})

const passwordInputRef = ref<any>(null)

// --- Auto Focus 邏輯 ---
watch(
  () => formData.value.passwordRequired,
  async (newVal) => {
    if (newVal) {
      await nextTick()
      passwordInputRef.value?.focus()
    } else {
      // 關閉時清空密碼
      formData.value.password = ''
    }
  }
)

// --- Expiration Toggle 邏輯 ---
const toggleExpiration = computed({
  get: () => formData.value.expireEnabled,
  set: (val: boolean) => {
    formData.value.expireEnabled = val

    if (val) {
      // 開啟時：如果沒有值，自動填入預設值
      if (!formData.value.expDuration) {
        formData.value.expDuration = DURATIONS[0]?.id || 60
      }
    } else {
      // 關閉時：清空數值
      formData.value.expDuration = null
    }
  }
})
</script>
