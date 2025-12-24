<template>
  <ShareModalBase
    v-model="modalStore.showShareModal"
    v-model:form-state="formState"
    title="Share Settings"
    mode="create"
    :share-link="shareLink"
    :loading="loading"
    @create="createLink"
    @update="updateLink"
  />
</template>

<script setup lang="ts">
import ShareModalBase from '@/components/Modal/ShareModalBase.vue'
import { ShareFormData } from '@/components/Modal/ShareSettingsForm.vue'
import { useModalStore } from '@/store/modalStore'
import { useMessageStore } from '@/store/messageStore'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import axios from 'axios'
import { ref, Ref, watch } from 'vue'

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

// 當 Modal 關閉時重置表單
watch(
  () => modalStore.showShareModal,
  (val) => {
    if (!val) {
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

// --- API 操作 ---
const createLink = async (formData: ShareFormData) => {
  loading.value = true
  const expirationTimestamp =
    formData.expireEnabled && formData.expDuration
      ? Math.floor(Date.now() / 1000) + formData.expDuration * 60
      : 0

  try {
    const result = await axios.post<string>('/post/create_share', {
      albumId: props.albumId,
      description: formData.description,
      password: formData.passwordRequired ? formData.password : null,
      showMetadata: formData.showMetadata,
      showDownload: formData.showDownload,
      showUpload: formData.showUpload,
      exp: expirationTimestamp
    })

    createdShareKey.value = result.data
    shareLink.value = `${window.location.origin}/share/${props.albumId}-${result.data}`
    messageStore.success('Share link created successfully.')
  } catch (e) {
    console.error(e)
    messageStore.error('Failed to create share link.')
  } finally {
    loading.value = false
  }
}

const updateLink = async (formData: ShareFormData) => {
  if (!createdShareKey.value) return

  loading.value = true
  const expirationTimestamp =
    formData.expireEnabled && formData.expDuration
      ? Math.floor(Date.now() / 1000) + formData.expDuration * 60
      : 0

  try {
    await tryWithMessageStore('mainId', async () => {
      await axios.put('/put/edit_share', {
        albumId: props.albumId,
        share: {
          url: createdShareKey.value,
          description: formData.description,
          password: formData.passwordRequired ? formData.password : null,
          showMetadata: formData.showMetadata,
          showDownload: formData.showDownload,
          showUpload: formData.showUpload,
          exp: expirationTimestamp
        }
      })
      messageStore.success('Share settings updated.')
    })
  } catch (e) {
    console.error('Update failed', e)
  } finally {
    loading.value = false
  }
}
</script>
