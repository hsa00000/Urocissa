<template>
  <v-dialog v-model="shareStore.isAuthFailed" max-width="400" persistent>
    <v-card rounded="xl">
      <v-card-title class="text-h5 pa-4"> Password Required </v-card-title>
      <v-card-text>
        <p class="mb-4 text-medium-emphasis">
          This share link is protected. Please enter the password to continue.
        </p>
        <v-text-field
          v-model="passwordInput"
          label="Password"
          type="password"
          variant="outlined"
          :error-messages="errorMessage"
          @keyup.enter="submit"
          autofocus
          :loading="loading"
          :disabled="loading"
        ></v-text-field>
      </v-card-text>
      <v-card-actions class="pa-4 pt-0">
        <v-spacer></v-spacer>
        <v-btn color="primary" variant="elevated" @click="submit" :loading="loading">
          Unlock
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import axios from 'axios'
import { useShareStore } from '@/store/shareStore'

const shareStore = useShareStore('mainId')
const passwordInput = ref('')
const errorMessage = ref('')
const loading = ref(false)

async function submit() {
  if (!passwordInput.value) return

  loading.value = true
  errorMessage.value = ''

  try {
    // 修改處：將 null 改為 {}
    // 這會強制 Axios 設定 Content-Type: application/json
    await axios.post(
      '/get/prefetch',
      {},
      {
        params: {
          locate: shareStore.albumId
        },
        headers: {
          'x-album-id': shareStore.albumId,
          'x-share-id': shareStore.shareId,
          'x-share-password': passwordInput.value
        }
      }
    )

    // 成功：更新 store 並關閉視窗
    shareStore.password = passwordInput.value
    shareStore.isAuthFailed = false
    errorMessage.value = ''
    passwordInput.value = ''
  } catch (error: any) {
    if (error.response) {
      const status = error.response.status

      // 只有 401 (Unauthorized) 或 403 (Forbidden) 代表密碼錯誤
      if (status === 401 || status === 403) {
        errorMessage.value = 'Incorrect password'
      }

      // 如果回傳 400 或 422 (Unprocessable Entity)，
      // 代表「密碼是對的 (通過了 Guard)」，但「我們傳的空物件 {} 格式不符」。
      // 這種情況下，我們其實應該算驗證成功，因為我們的目的只是測密碼。
      else if (status === 400 || status === 422) {
        shareStore.password = passwordInput.value
        shareStore.isAuthFailed = false
        errorMessage.value = ''
        passwordInput.value = ''
      } else {
        errorMessage.value = 'Server error. Please try again.'
      }
    } else {
      errorMessage.value = 'Network error.'
    }
  } finally {
    loading.value = false
  }
}
</script>
