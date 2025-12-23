<template>
  <BaseModal v-model="modalStore.showShareModal" title="Share Settings" width="450">
    <ShareSettingsForm v-model="formState" />

    <template #actions>
      <v-sheet
        border
        :color="shareLink ? 'grey-darken-4' : 'transparent'"
        :style="{
          borderColor: shareLink ? 'rgba(255,255,255,0.15)' : 'transparent !important',
          transition: 'none !important'
        }"
        :class="['d-flex align-center w-100 pr-1', shareLink ? 'pl-4' : 'justify-end']"
        height="54"
      >
        <div
          v-if="shareLink"
          class="text-body-2 text-grey-lighten-1 text-truncate flex-grow-1 mr-3"
          style="user-select: all"
        >
          {{ shareLink }}
        </div>

        <v-btn
          color="primary"
          variant="flat"
          width="150"
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
import BaseModal from '@/components/Modal/BaseModal.vue'
import ShareSettingsForm, { ShareFormData } from '@/components/Modal/ShareSettingsForm.vue'
import { useModalStore } from '@/store/modalStore'
import { useMessageStore } from '@/store/messageStore'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import axios from 'axios'
import { ref, Ref, computed, watch } from 'vue'
import { useClipboard } from '@vueuse/core'

const props = defineProps<{
  albumId: string
}>()

const modalStore = useModalStore('mainId')
const messageStore = useMessageStore('mainId')

// 初始狀態
const defaultFormState: ShareFormData = {
  description: '',
  passwordRequired: false,
  password: '',
  expireEnabled: false,
  expDuration: null,
  showUpload: false,
  showDownload: true,
  showMetadata: false
}

const formState = ref<ShareFormData>({ ...defaultFormState })

// 當 Modal 關閉時重置表單（可選）
watch(
  () => modalStore.showShareModal,
  (val) => {
    if (!val && !shareLink.value) {
      // 只有在沒有生成連結的情況下才完全重置，或者每次打開都重置
      formState.value = { ...defaultFormState }
      createdShareKey.value = null
      shareLink.value = null
    }
  }
)

// 狀態資料
const shareLink: Ref<string | null> = ref(null)
const createdShareKey: Ref<string | null> = ref(null)
const loading = ref(false)
const lastSavedState = ref('')

// Clipboard logic
const { copy, copied } = useClipboard({ legacy: true })

// --- 驗證與變更偵測 ---
const isFormValid = computed(() => {
  if (formState.value.passwordRequired && !formState.value.password) return false
  return true
})

const hasChanges = computed(() => {
  if (!createdShareKey.value) return true
  return JSON.stringify(formState.value) !== lastSavedState.value
})

// --- 按鈕邏輯 ---
const buttonLabel = computed(() => {
  if (!shareLink.value) return 'Create Link'
  if (hasChanges.value) return 'Save Changes'
  return copied.value ? 'Copied!' : 'Copy'
})

const handleAction = () => {
  if (!shareLink.value) {
    createLink()
  } else if (hasChanges.value) {
    updateLink()
  } else {
    performCopy(shareLink.value)
  }
}

// --- API 操作 ---
const createLink = async () => {
  if (!isFormValid.value) return
  loading.value = true
  const expirationTimestamp =
    formState.value.expireEnabled && formState.value.expDuration
      ? Math.floor(Date.now() / 1000) + formState.value.expDuration * 60
      : 0

  try {
    const result = await axios.post<string>('/post/create_share', {
      albumId: props.albumId,
      description: formState.value.description,
      password: formState.value.passwordRequired ? formState.value.password : null,
      showMetadata: formState.value.showMetadata,
      showDownload: formState.value.showDownload,
      showUpload: formState.value.showUpload,
      exp: expirationTimestamp
    })

    createdShareKey.value = result.data
    shareLink.value = `${window.location.origin}/share/${props.albumId}-${result.data}`
    lastSavedState.value = JSON.stringify(formState.value)
    messageStore.success('Share link created successfully.')
  } catch (e) {
    console.error(e)
    messageStore.error('Failed to create share link.')
  } finally {
    loading.value = false
  }
}

const updateLink = async () => {
  if (!createdShareKey.value) return
  if (!isFormValid.value) return

  loading.value = true
  const expirationTimestamp =
    formState.value.expireEnabled && formState.value.expDuration
      ? Math.floor(Date.now() / 1000) + formState.value.expDuration * 60
      : 0

  try {
    await tryWithMessageStore('mainId', async () => {
      await axios.put('/put/edit_share', {
        albumId: props.albumId,
        share: {
          url: createdShareKey.value,
          description: formState.value.description,
          password: formState.value.passwordRequired ? formState.value.password : null,
          showMetadata: formState.value.showMetadata,
          showDownload: formState.value.showDownload,
          showUpload: formState.value.showUpload,
          exp: expirationTimestamp
        }
      })
      lastSavedState.value = JSON.stringify(formState.value)
      messageStore.success('Share settings updated.')
    })
  } catch (e) {
    console.error('Update failed', e)
  } finally {
    loading.value = false
  }
}

async function performCopy(text: string) {
  if (!text) return
  await copy(text)
  messageStore.success('Link copied to clipboard')
}
</script>
