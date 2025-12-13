<template>
  <v-dialog v-model="modalStore.showShareLoginModal" persistent max-width="400">
    <v-card class="mx-auto w-100" variant="elevated" retain-focus rounded="xl">
      <template #title>
        {{ shareStore.isLinkExpired ? 'Link Expired' : 'Password Required' }}
      </template>

      <template #text>
        <div v-if="shareStore.isLinkExpired">
          This share link has expired and is no longer accessible.
        </div>

        <div v-else>
          <div>This share link is protected. Please enter the password to continue.</div>
          <v-text-field
            v-model="password"
            label="Password"
            type="password"
            variant="outlined"
            :error-messages="errorMessage"
            @keyup.enter="submit"
            autofocus
            :loading="loading"
            :disabled="loading"
            class="mt-3"
          ></v-text-field>
        </div>
      </template>

      <template v-if="!shareStore.isLinkExpired">
        <v-divider />

        <v-card-actions>
          <v-spacer />
          <v-btn color="primary" variant="elevated" @click="submit" :loading="loading">
            Unlock
          </v-btn>
        </v-card-actions>
      </template>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useModalStore } from '@/store/modalStore'
import { useMessageStore } from '@/store/messageStore'
import { useShareStore } from '@/store/shareStore'
import axios from 'axios'

const modalStore = useModalStore('mainId')
const messageStore = useMessageStore('mainId')
const shareStore = useShareStore('mainId')

const password = ref('')

const errorMessage = ref('')
const loading = ref(false)

watch(
  () => modalStore.showShareLoginModal,
  (val) => {
    if (val) {
      errorMessage.value = ''
      password.value = ''
      loading.value = false
    }
  }
)

const submit = async () => {
  if (!password.value) return

  loading.value = true
  errorMessage.value = ''

  try {
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
          'x-share-password': password.value
        }
      }
    )

    shareStore.password = password.value
    errorMessage.value = ''
    password.value = ''
    modalStore.showShareLoginModal = false
  } catch (error: any) {
    const status = error.response?.status

    if (status === 403) {
      shareStore.isLinkExpired = true
      messageStore.error('Link has expired.')
    } else if (status === 401) {
      errorMessage.value = 'Incorrect password'
    } else {
      errorMessage.value = 'Server error or invalid request.'
    }
  } finally {
    loading.value = false
  }
}
</script>
