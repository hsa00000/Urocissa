<template>
  <v-dialog
    v-if="submit !== undefined"
    v-model="modalStore.showEditShareModal"
    id="share-modal"
    variant="flat"
    persistent
    rounded
  >
    <v-confirm-edit
      v-model="shareModel"
      :disabled="false"
      @save="submit"
      @cancel="modalStore.showEditShareModal = false"
    >
      <template #default="{ model: proxyModel, actions }">
        <v-card
          class="h-100 mx-auto w-100"
          max-width="400"
          variant="elevated"
          retain-focus
          rounded="xl"
        >
          <v-toolbar color="transparent">
            <v-toolbar-title class="text-h5">Share</v-toolbar-title>
            <template #append>
              <v-btn icon="mdi-close" @click="modalStore.showEditShareModal = false" />
            </template>
          </v-toolbar>
          <v-divider />
          <v-list class="px-6">
            <v-list-item>
              <v-textarea
                v-model="proxyModel.value.description"
                label="Description of this link"
                hide-details="auto"
                rows="1"
              />
            </v-list-item>

            <v-list-item density="compact">
              <v-text-field
                v-model="proxyModel.value.password"
                label="Password (Optional)"
                placeholder="Leave empty for no password"
                hide-details="auto"
                clearable
              ></v-text-field>
            </v-list-item>
          </v-list>
          <v-divider />
          <v-list class="px-6">
            <v-list-item density="compact">
              <template #prepend>
                <v-list-item-action start>
                  <v-switch
                    v-model="proxyModel.value.showDownload"
                    color="primary"
                    :label="`Allow public user to download`"
                    hide-details
                  />
                </v-list-item-action>
              </template>
            </v-list-item>
            <v-list-item density="compact">
              <template #prepend>
                <v-list-item-action start>
                  <v-switch
                    v-model="proxyModel.value.showUpload"
                    color="primary"
                    :label="`Allow public user to upload`"
                    hide-details
                  />
                </v-list-item-action>
              </template>
            </v-list-item>
            <v-list-item density="compact">
              <template #prepend>
                <v-list-item-action start>
                  <v-switch
                    v-model="proxyModel.value.showMetadata"
                    color="primary"
                    :label="`Show metadata`"
                    hide-details
                  />
                </v-list-item-action>
              </template>
            </v-list-item>
          </v-list>

          <v-divider />

          <v-list class="px-6">
            <v-list-item density="compact">
              <v-list-item-title class="text-caption mb-1">
                Expires:
                {{
                  proxyModel.value.exp === 0
                    ? 'Never'
                    : new Date(proxyModel.value.exp * 1000).toLocaleString()
                }}
              </v-list-item-title>
              <v-select
                :model-value="newDuration"
                @update:model-value="(val: number | null) => updateExpiration(proxyModel, val)"
                :items="DURATIONS"
                label="Reset Expiration to..."
                item-title="label"
                item-value="id"
                hide-details="auto"
                clearable
                persistent-hint
                hint="Select to update expiration time from now"
              />
            </v-list-item>
          </v-list>

          <template #actions>
            <v-spacer />
            <component :is="actions"></component>
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
import type { EditShareData, Share } from '@/type/types'
import { useMessageStore } from '@/store/messageStore'
import { useAlbumStore } from '@/store/albumStore'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import { DURATIONS } from '@type/constants'

const props = defineProps<{ editShareData: EditShareData }>()

const modalStore = useModalStore('mainId')
const messageStore = useMessageStore('mainId')
const albumStore = useAlbumStore('mainId')

const shareModel = ref<Share>({
  url: props.editShareData.share.url,
  description: props.editShareData.share.description,
  showDownload: props.editShareData.share.showDownload,
  showUpload: props.editShareData.share.showUpload,
  showMetadata: props.editShareData.share.showMetadata,
  exp: props.editShareData.share.exp,
  password: props.editShareData.share.password
})

const newDuration = ref<number | null>(null)
const submit = ref<(() => Promise<void>) | undefined>()

// Helper function to handle expiration time update
const updateExpiration = (proxyModel: any, val: number | null) => {
  newDuration.value = val
  if (val !== null) {
    // Calculate new expiration time from now
    proxyModel.value.exp = Math.floor(Date.now() / 1000) + val * 60
  } else {
    proxyModel.value.exp = 0
  }
}

onMounted(() => {
  submit.value = async () => {
    modalStore.showEditShareModal = false

    if (shareModel.value.password === '') {
      shareModel.value.password = null
    }

    // Update props.editShareData.share for optimistic update in LinksPage
    Object.assign(props.editShareData.share, shareModel.value)

    // Update AlbumStore if the album exists
    const album = albumStore.albums.get(props.editShareData.albumId)
    if (album) {
      album.shareList.set(props.editShareData.share.url, shareModel.value)
    }

    await tryWithMessageStore('mainId', async () => {
      await axios.put('/put/edit_share', {
        albumId: props.editShareData.albumId,
        share: shareModel.value
      })

      messageStore.success('Updated share settings successfully')
    })
  }
})
</script>
