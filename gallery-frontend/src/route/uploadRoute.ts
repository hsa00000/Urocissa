import type { RouteRecordRaw } from 'vue-router'
import UploadPage from '@/components/Page/UploadPage.vue'

export const uploadRoute: RouteRecordRaw = {
  path: '/upload',
  component: UploadPage,
  name: 'upload',
  meta: {
    level: 1,
    baseName: 'upload',
    getParentPage: (route) => ({
      name: 'home',
      params: { hash: undefined, subhash: undefined },
      query: route.query
    }),
    getChildPage: (route) => ({
      name: 'upload',
      params: { hash: undefined, subhash: undefined },
      query: route.query
    })
  }
}
