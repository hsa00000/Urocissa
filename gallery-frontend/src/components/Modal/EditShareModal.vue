<template>
  <BaseModal
    v-if="modalStore.showEditShareModal"
    v-model="modalStore.showEditShareModal"
    title="Edit Share Settings"
    width="450"
  >
    <ShareSettingsForm v-model="formState" duration-label="Reset Duration" />

    <template #actions>
      <v-sheet class="d-flex justify-end w-100 pa-2" color="transparent">
        <v-btn
          color="primary"
          variant="flat"
          width="150"
          height="44"
          class="text-capitalize"
          :loading="loading"
          :disabled="!isFormValid"
          @click="saveChanges"
        >
          Save Changes
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
import { useAlbumStore } from '@/store/albumStore'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import { EditShareData } from '@/type/types'
import axios from 'axios'
import { ref, computed, watchEffect } from 'vue'

const props = defineProps<{ editShareData: EditShareData }>()

const modalStore = useModalStore('mainId')
const messageStore = useMessageStore('mainId')
const albumStore = useAlbumStore('mainId')

const loading = ref(false)

// 初始化表單狀態
const formState = ref<ShareFormData>({
  description: '',
  passwordRequired: false,
  password: '',
  expireEnabled: false,
  expDuration: null,
  showUpload: false,
  showDownload: false,
  showMetadata: false
})

// 監聽 props 變更以初始化資料
watchEffect(() => {
  if (props.editShareData && props.editShareData.share) {
    const share = props.editShareData.share
    formState.value = {
      description: share.description || '',
      passwordRequired: !!share.password,
      password: share.password || '',
      // 如果 exp > 0 代表有過期時間
      expireEnabled: share.exp > 0,
      // 編輯模式下，duration 預設為 null (不改變)，除非使用者想重設
      expDuration: null,
      showUpload: share.showUpload,
      showDownload: share.showDownload,
      showMetadata: share.showMetadata
    }
  }
})

const isFormValid = computed(() => {
  if (formState.value.passwordRequired && !formState.value.password) return false
  return true
})

const saveChanges = async () => {
  if (!isFormValid.value) return
  loading.value = true

  // 計算新的過期時間
  let newExp = props.editShareData.share.exp

  if (!formState.value.expireEnabled) {
    // 使用者關閉了過期
    newExp = 0
  } else if (formState.value.expDuration) {
    // 使用者選擇了新的時長，重設過期時間
    newExp = Math.floor(Date.now() / 1000) + formState.value.expDuration * 60
  }
  // 如果 enabled 為 true 但 duration 為 null，則保持原有的 exp 不變

  const updatedShare = {
    url: props.editShareData.share.url,
    description: formState.value.description,
    password: formState.value.passwordRequired ? formState.value.password : null,
    showMetadata: formState.value.showMetadata,
    showDownload: formState.value.showDownload,
    showUpload: formState.value.showUpload,
    exp: newExp
  }

  try {
    // Optimistic Update (更新 Store)
    const album = albumStore.albums.get(props.editShareData.albumId)
    if (album) {
      // 這裡需要確保 editShareData.share 的引用被更新，
      // 或者更新 album store 中的 Map
      Object.assign(props.editShareData.share, updatedShare)
      album.shareList.set(updatedShare.url, { ...props.editShareData.share, ...updatedShare })
    }

    await tryWithMessageStore('mainId', async () => {
      await axios.put('/put/edit_share', {
        albumId: props.editShareData.albumId,
        share: updatedShare
      })
      messageStore.success('Updated share settings successfully')
      modalStore.showEditShareModal = false
    })
  } catch (e) {
    console.error('Update failed', e)
  } finally {
    loading.value = false
  }
}
</script>
