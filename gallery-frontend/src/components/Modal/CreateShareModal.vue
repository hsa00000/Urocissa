<template>
  <BaseModal v-model="modalStore.showShareModal" title="Share Settings" width="450">
    <v-textarea
      v-model="description"
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
          v-model="showDownload"
          label="Allow Download"
          color="primary"
          density="compact"
          hide-details
          inset
        ></v-switch>
      </v-col>
      <v-col cols="6">
        <v-switch
          v-model="showUpload"
          label="Allow Upload"
          color="primary"
          density="compact"
          hide-details
          inset
        ></v-switch>
      </v-col>
      <v-col cols="12">
        <v-switch
          v-model="showMetadata"
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
          v-model="passwordRequired"
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
          v-model="password"
          :disabled="!passwordRequired"
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
          v-model="expireEnabled"
          label="Expiration"
          color="primary"
          density="compact"
          hide-details
          inset
        ></v-switch>
      </v-col>
      <v-col cols="7">
        <v-select
          v-model="exp"
          :items="DURATIONS"
          :disabled="!expireEnabled"
          label="Duration"
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
        rounded="pill"
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
          rounded="pill"
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
import { useModalStore } from '@/store/modalStore'
import { useMessageStore } from '@/store/messageStore'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import axios from 'axios'
import { ref, Ref, watchEffect, computed, watch, nextTick } from 'vue'
import { useClipboard } from '@vueuse/core'
import { DURATIONS } from '@type/constants'

const props = defineProps<{
  albumId: string
}>()

const modalStore = useModalStore('mainId')
const messageStore = useMessageStore('mainId')

// 表單資料
const description = ref('')
const passwordRequired = ref(false)
const password = ref('')
const expireEnabled = ref(false)
const showUpload = ref(false)
const showDownload = ref(true)
const showMetadata = ref(false)
const exp: Ref<number | null> = ref(null)

// 狀態資料
const shareLink: Ref<string | null> = ref(null)
const createdShareKey: Ref<string | null> = ref(null)
const loading = ref(false)
const lastSavedState = ref('')

// DOM 引用
const passwordInputRef = ref<any>(null)

// Clipboard logic
const { copy, copied } = useClipboard({ legacy: true })

// 清理邏輯
watchEffect(() => {
  if (!passwordRequired.value) password.value = ''
  if (!expireEnabled.value) exp.value = null
})

// --- Auto Focus 邏輯 ---
watch(passwordRequired, async (newVal) => {
  if (newVal) {
    await nextTick()
    passwordInputRef.value?.focus()
  }
})

// --- 驗證與變更偵測 ---
const isFormValid = computed(() => {
  if (passwordRequired.value && !password.value) return false
  return true
})

const currentFormState = computed(() => ({
  description: description.value,
  passwordRequired: passwordRequired.value,
  password: password.value,
  expireEnabled: expireEnabled.value,
  exp: exp.value,
  showUpload: showUpload.value,
  showDownload: showDownload.value,
  showMetadata: showMetadata.value
}))

const hasChanges = computed(() => {
  if (!createdShareKey.value) return true
  return JSON.stringify(currentFormState.value) !== lastSavedState.value
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
    expireEnabled.value && exp.value ? Math.floor(Date.now() / 1000) + exp.value * 60 : 0

  try {
    const result = await axios.post<string>('/post/create_share', {
      albumId: props.albumId,
      description: description.value,
      password: passwordRequired.value ? password.value : null,
      showMetadata: showMetadata.value,
      showDownload: showDownload.value,
      showUpload: showUpload.value,
      exp: expirationTimestamp
    })

    createdShareKey.value = result.data
    shareLink.value = `${window.location.origin}/share/${props.albumId}-${result.data}`
    lastSavedState.value = JSON.stringify(currentFormState.value)
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
    expireEnabled.value && exp.value ? Math.floor(Date.now() / 1000) + exp.value * 60 : 0

  try {
    await tryWithMessageStore('mainId', async () => {
      await axios.put('/put/edit_share', {
        albumId: props.albumId,
        share: {
          url: createdShareKey.value,
          description: description.value,
          password: passwordRequired.value ? password.value : null,
          showMetadata: showMetadata.value,
          showDownload: showDownload.value,
          showUpload: showUpload.value,
          exp: expirationTimestamp
        }
      })
      lastSavedState.value = JSON.stringify(currentFormState.value)
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
