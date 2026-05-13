<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon>mdi-camera-iris</v-icon>
      </v-avatar>
    </template>
    <v-list-item-title :class="metadataTextClass">{{ exifMake }}</v-list-item-title>
    <v-list-item-subtitle>
      <div class="d-flex flex-wrap ga-2">
        <span>{{ formattedExif.FNumber }}</span>
        <span>{{ formattedExif.ExposureTime }}</span>
        <span>{{ formattedExif.FocalLength }}</span>
        <span>{{ formattedExif.PhotographicSensitivity }}</span>
      </div>
    </v-list-item-subtitle>
  </v-list-item>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { GalleryImage, GalleryVideo } from '@type/types'
import { useMetadataItemLayout } from './useMetadataItemLayout'

const props = defineProps<{
  database: GalleryImage | GalleryVideo
}>()

const { metadataTextClass } = useMetadataItemLayout()
const formattedExif = computed(() => formatExifData(props.database.exif))
const exifMake = computed(() => generateExifMake(props.database.exif))

function generateExifMake(exifData: Record<string, string>): string {
  let make_formated = ''
  let model_formated = ''
  if (exifData.Make !== undefined) {
    const make: string = exifData.Make.replace(/"/g, '')
    make_formated = make
      .split(',')
      .map((part) => part.trim())
      .filter((part) => part !== '')
      .join(', ')
  }
  if (exifData.Model !== undefined) {
    const model: string = exifData.Model.replace(/"/g, '')
    model_formated = model
      .split(',')
      .map((part) => part.trim())
      .filter((part) => part !== '')
      .join(', ')
  }
  return make_formated + ' ' + model_formated
}

interface ExifData {
  FNumber: string // Aperture value as a string, e.g., "f/2.8"
  ExposureTime: string // Exposure time as a string, e.g., "1/60 s"
  FocalLength: string // Focal length as a string, e.g., "35 mm"
  PhotographicSensitivity: string
}

function formatExifData(exifData: Record<string, string | undefined>): ExifData {
  const formattedExifData: ExifData = {
    FNumber: exifData.FNumber !== undefined ? exifData.FNumber.replace('f/', 'ƒ/') : '',
    ExposureTime:
      exifData.ExposureTime !== undefined
        ? `1/${exifData.ExposureTime.replace(' s', '').replace('1/', '')}`
        : '',
    FocalLength:
      exifData.FocalLength !== undefined ? `${exifData.FocalLength.replace(' mm', '')} mm` : '',
    PhotographicSensitivity:
      exifData.PhotographicSensitivity !== undefined
        ? `ISO ${exifData.PhotographicSensitivity}`
        : ''
  }

  return formattedExifData
}
</script>
