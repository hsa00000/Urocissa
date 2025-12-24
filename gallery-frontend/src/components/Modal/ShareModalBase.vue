<template>
  <BaseModal
    v-model="modelValue"
    :title="title"
    :width="mobile ? '100%' : 450"
    :fullscreen="mobile"
  >
    <ShareSettingsForm v-model="formState" />

    <template #actions>
      <v-sheet
        border
        :color="showLinkDisplay ? 'grey-darken-4' : 'transparent'"
        :style="{
          borderColor: showLinkDisplay ? 'rgba(255,255,255,0.15)' : 'transparent !important',
          transition: 'none !important'
        }"
        :class="[
          'd-flex w-100',
          mobile ? 'flex-column pa-2' : 'align-center pr-1',
          showLinkDisplay && !mobile ? 'pl-4' : '',
          !showLinkDisplay && !mobile ? 'justify-end pa-2' : ''
        ]"
        :height="mobile ? 'auto' : 54"
      >
        <!-- 顯示已建立的連結 -->
        <div
          v-if="showLinkDisplay && !mobile"
          class="text-body-2 text-grey-lighten-1 text-truncate flex-grow-1 mr-3"
          style="user-select: all"
        >
          {{ shareLink }}
        </div>

        <!-- Mobile 版連結顯示 -->
        <v-sheet
          v-if="showLinkDisplay && mobile"
          color="grey-darken-4"
          class="pa-3 mb-2 rounded text-body-2 text-grey-lighten-1"
          style="user-select: all; word-break: break-all"
        >
          {{ shareLink }}
        </v-sheet>

        <v-btn
          color="primary"
          variant="flat"
          :width="mobile ? '100%' : 150"
          height="44"
          class="text-capitalize"
          :loading="loading"
          :disabled="!isFormValid"
          @click="handleAction"
        >
          {{ buttonLabel }}
        </v-btn>
      </v-sheet>
    </template>
  </BaseModal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useDisplay } from 'vuetify'
import { useClipboard } from '@vueuse/core'
import BaseModal from '@/components/Modal/BaseModal.vue'
import ShareSettingsForm, { ShareFormData } from '@/components/Modal/ShareSettingsForm.vue'
import { useMessageStore } from '@/store/messageStore'

// Props
const props = withDefaults(
  defineProps<{
    /** Modal 標題 */
    title: string
    /** 已建立的分享連結（若有） */
    shareLink?: string | null
    /** 外部控制的 loading 狀態 */
    loading?: boolean
    /** 模式：'create' = 建立新連結, 'edit' = 編輯現有連結 */
    mode: 'create' | 'edit'
  }>(),
  {
    shareLink: null,
    loading: false
  }
)

// Emits
const emit = defineEmits<{
  /** 建立連結 */
  create: [formData: ShareFormData]
  /** 更新連結 */
  update: [formData: ShareFormData]
  /** 複製連結 */
  copy: [link: string]
}>()

// Models
const modelValue = defineModel<boolean>({ required: true })
const formState = defineModel<ShareFormData>('formState', { required: true })

// 取得 mobile 狀態
const { mobile } = useDisplay()

// Clipboard
const { copy, copied } = useClipboard({ legacy: true })
const messageStore = useMessageStore('mainId')

// --- 內部狀態追蹤 ---
const lastSavedState = ref('')

// 當有連結產生時，儲存當前狀態
watch(
  () => props.shareLink,
  (newLink) => {
    if (newLink) {
      lastSavedState.value = JSON.stringify(formState.value)
    }
  }
)

// --- Computed ---
const showLinkDisplay = computed(() => !!props.shareLink)

const isFormValid = computed(() => {
  if (formState.value.passwordRequired && !formState.value.password) return false
  return true
})

const hasChanges = computed(() => {
  if (!props.shareLink) return true
  return JSON.stringify(formState.value) !== lastSavedState.value
})

const buttonLabel = computed(() => {
  if (props.mode === 'edit') {
    return 'Save Changes'
  }
  // Create mode
  if (!props.shareLink) return 'Create Link'
  if (hasChanges.value) return 'Save Changes'
  return copied.value ? 'Copied!' : 'Copy'
})

// --- Actions ---
const handleAction = async () => {
  if (props.mode === 'edit') {
    emit('update', { ...formState.value })
    return
  }

  // Create mode
  if (!props.shareLink) {
    emit('create', { ...formState.value })
  } else if (hasChanges.value) {
    emit('update', { ...formState.value })
    lastSavedState.value = JSON.stringify(formState.value)
  } else {
    // Copy action
    if (props.shareLink) {
      await copy(props.shareLink)
      messageStore.success('Link copied to clipboard')
      emit('copy', props.shareLink)
    }
  }
}

// 暴露方法給父組件使用
defineExpose({
  /** 標記當前狀態為已儲存（用於成功儲存後） */
  markAsSaved: () => {
    lastSavedState.value = JSON.stringify(formState.value)
  }
})
</script>
