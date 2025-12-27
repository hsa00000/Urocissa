<template>
  <v-dialog
    v-if="submit !== undefined"
    v-model="modalStore.showDeleteShareModal"
    id="delete-share-modal"
    persistent
    variant="flat"
    rounded
  >
    <v-confirm-edit
      v-model="dummy"
      :disabled="false"
      @save="submit"
      @cancel="modalStore.showDeleteShareModal = false"
    >
      <template #default="{ actions }">
        <v-card class="mx-auto w-100" max-width="400" variant="elevated" rounded="xl" retain-focus>
          <template #title>Delete&nbsp;Link</template>

          <template #text>
            <div class="pa-4">
              <p class="mb-2">Are you sure you want to delete this share link?</p>
              <p class="text-medium-emphasis text-truncate">
                {{ shareUrl }}
              </p>
            </div>
          </template>

          <v-divider />

          <template #actions>
            <v-spacer />
            <component :is="actions" />
          </template>
        </v-card>
      </template>
    </v-confirm-edit>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import axios from 'axios'
import { useModalStore } from '@/store/modalStore'
import { useAlbumStore } from '@/store/albumStore'
import { useMessageStore } from '@/store/messageStore'
import type { EditShareData } from '@/type/types'
import { tryWithMessageStore } from '@/script/utils/try_catch'

const props = defineProps<{ deleteShareData: EditShareData }>()

const modalStore = useModalStore('mainId')
const albumStore = useAlbumStore('mainId')
const messageStore = useMessageStore('mainId')

const dummy = ref(true) // v-confirm-edit 需要的綁定值
const shareUrl = `${window.location.origin}/share/${props.deleteShareData.albumId}-${props.deleteShareData.share.url}`

const submit = ref<(() => void) | undefined>()

onMounted(() => {
  submit.value = () => {
    void tryWithMessageStore('mainId', async () => {
      await axios.put('/put/delete_share', {
        albumId: props.deleteShareData.albumId,
        shareId: props.deleteShareData.share.url
      })

      messageStore.success('Share deleted')
      modalStore.showDeleteShareModal = false
      await albumStore.fetchAlbums() // 重新載入列表
    })
  }
})
</script>
