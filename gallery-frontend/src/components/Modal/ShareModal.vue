<template>
  <v-dialog v-model="modalStore.showShareModal" id="share-modal" variant="flat" persistent rounded>
    <v-card
      class="h-100 mx-auto w-100"
      max-width="400"
      variant="elevated"
      retain-focus
      rounded="xl"
    >
      <v-toolbar color="transparent">
        <v-toolbar-title class="text-h5" text=" Share"></v-toolbar-title>
        <template #append>
          <v-btn icon="mdi-close" @click="modalStore.showShareModal = false"></v-btn>
        </template>
      </v-toolbar>
      <v-divider></v-divider>

      <v-list class="px-6" density="compact">
        <v-list-item>
          <v-textarea
            v-model="description"
            label="Description of this link"
            hide-details="auto"
            :style="{
              paddingBottom: 0
            }"
          ></v-textarea>
        </v-list-item>
        <v-list-item v-if="false" density="compact" slim>
          <template #prepend>
            <v-list-item-action start>
              <v-switch
                v-model="requirePassword"
                color="primary"
                :label="`Require password`"
                hide-details
              ></v-switch>
            </v-list-item-action>
          </template>
        </v-list-item>
        <v-list-item v-if="false" density="compact" slim>
          <v-text-field
            v-model="password"
            label="Password"
            hide-details="auto"
            :disabled="!requirePassword"
            :style="{
              paddingBottom: 0
            }"
          ></v-text-field>
        </v-list-item>
        <v-list-item density="compact" slim>
          <template #prepend>
            <v-list-item-action start>
              <v-switch
                v-model="allowDownload"
                color="primary"
                :label="`Allow public user to download`"
                hide-details
              ></v-switch>
            </v-list-item-action>
          </template>
        </v-list-item>
        <v-list-item v-if="false" density="compact" slim>
          <template #prepend>
            <v-list-item-action start>
              <v-switch
                v-model="allowUpload"
                color="primary"
                :label="`Allow public user to upload`"
                hide-details
              ></v-switch>
            </v-list-item-action>
          </template>
        </v-list-item>
        <v-list-item v-if="false" density="compact" slim>
          <template #prepend>
            <v-list-item-action start>
              <v-switch
                v-model="showMetadata"
                color="primary"
                :label="`Show metadata`"
                hide-details
              ></v-switch>
            </v-list-item-action>
          </template>
        </v-list-item>
        <v-list-item v-if="false" density="compact" slim>
          <template #prepend>
            <v-list-item-action start>
              <v-switch
                v-model="willExpire"
                color="primary"
                :label="`Expire after`"
                hide-details
              ></v-switch>
            </v-list-item-action>
          </template>
        </v-list-item>
        <v-list-item v-if="false" density="compact" slim>
          <v-select
            v-model="selectedDuration"
            :items="durations"
            label="Select a duration"
            item-title="label"
            item-value="id"
            hide-details="auto"
            :disabled="!willExpire"
          />
        </v-list-item>
        <v-list-item density="compact" slim class="py-6">
          <v-card height="40px">
            <v-btn
              v-if="shareLink === null"
              color="teal-accent-4"
              variant="outlined"
              class="button button-submit"
              type="submit"
              block
              @click="createLink()"
            >
              Create Link
            </v-btn>
            <v-text-field
              v-else
              rounded
              v-model="shareLink"
              slim
              density="compact"
              variant="outlined"
              readonly
              append-inner-icon="mdi-content-copy"
              @click:append-inner="performCopy(shareLink)"
              hide-details
            >
            </v-text-field>
          </v-card>
        </v-list-item>
      </v-list>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { useModalStore } from '@/store/modalStore'
import axios from 'axios'
import { Ref, ref, watchEffect } from 'vue'
import { useClipboard } from '@vueuse/core'
import { useMessageStore } from '@/store/messageStore'

const props = defineProps<{
  albumId: string
}>()

const modalStore = useModalStore('mainId')
const messageStore = useMessageStore('mainId')
const description = ref('')
const requirePassword = ref(false)
const password = ref('')
const willExpire = ref(false)
const allowUpload = ref(false)
const allowDownload = ref(true)
const showMetadata = ref(true)
const shareLink: Ref<string | null> = ref(null)
const selectedDuration = ref<number | null>(null)

const { copy } = useClipboard()

const durations = [
  { label: '30 minutes later', id: 30 },
  { label: '1 hour later', id: 60 },
  { label: '6 hours later', id: 360 },
  { label: '1 day later', id: 1440 },
  { label: '7 days later', id: 10080 },
  { label: '30 days later', id: 43200 },
  { label: '3 months later', id: 129600 },
  { label: '1 year later', id: 525600 }
]

watchEffect(() => {
  console.log('selectedDuration is', selectedDuration.value)
  console.log('description is', description.value)
})

const createLink = async () => {
  const result = await axios.post<string>('/post/create_share', {
    albumId: props.albumId,
    description: description.value,
    password: requirePassword.value ? password.value : null,
    showMetadata: showMetadata.value,
    showDownload: allowDownload.value,
    showUpload: allowUpload.value,
    exp: selectedDuration.value ?? 0
  })
  shareLink.value = `${window.location.origin}/share-${props.albumId}-${result.data}`
  console.log('shareLink is', shareLink)
}

async function performCopy(text: string) {
  await copy(text)
  messageStore.success('Url copied')
}
</script>
