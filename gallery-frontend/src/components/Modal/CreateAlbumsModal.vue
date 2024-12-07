<template>
  <v-dialog
    v-model="modalStore.showCreateAlbumsModal"
    variant="flat"
    persistent
    id="edit-tag-overlay"
  >
    <v-card class="mx-auto w-100" max-width="400" variant="elevated">
      <v-form v-model="formIsValid" @submit.prevent="createAlbum" validate-on="input">
        <v-card-title class="text-h5">Create Albums</v-card-title>
        <v-container>
          <v-text-field
            v-model="albumName"
            :rules="[rules.required]"
            label="Album Name"
          ></v-text-field>
        </v-container>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn
            color="grey-lighten-2"
            variant="text"
            class="ma-2 button button-submit"
            @click="closeModal"
          >
            Cancel
          </v-btn>
          <v-btn
            color="teal-accent-4"
            variant="outlined"
            class="ma-2 button button-submit"
            :disabled="!formIsValid"
            type="submit"
          >
            Submit
          </v-btn>
        </v-card-actions>
      </v-form>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import axios from 'axios'
import { useMessageStore } from '@/store/messageStore'
import { useModalStore } from '@/store/modalStore'

const modalStore = useModalStore('mainId')
const messageStore = useMessageStore('mainId')

const albumName = ref<string>('')
const formIsValid = ref<boolean>(false)

const rules = {
  required: (value: string) => !!value || 'Album Name is required'
}

const createAlbum = async () => {
  try {
    const createAlbumData = {
      title: albumName.value,
      elements: []
    }

    const response = await axios.post('/post/create_album', createAlbumData, {
      headers: {
        'Content-Type': 'application/json'
      }
    })

    console.log(response)

    messageStore.message = 'Album created successfully.'
    messageStore.warn = false
    messageStore.showMessage = true

    modalStore.showCreateAlbumsModal = false
  } catch (error) {
    console.error('Error creating album:', error)
    messageStore.message = 'Failed to create album.'
    messageStore.warn = true
    messageStore.showMessage = true
  }
}

const closeModal = () => {
  modalStore.showCreateAlbumsModal = false
}
</script>

<style scoped></style>
