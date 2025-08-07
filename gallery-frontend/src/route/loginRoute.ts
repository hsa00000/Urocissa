// src/router.ts

import { RouteRecordRaw } from 'vue-router'
import 'vue-router'

import LoginPage from '@/components/Page/LoginPage.vue'
import { useFilterStringStore } from '@/store/filterStringStore'

export const loginRoute: RouteRecordRaw = {
  path: '/login',
  component: LoginPage,
  name: 'login',
  beforeEnter: () => {
    const filterStringStore = useFilterStringStore()
    filterStringStore.filterString = null
  },
  meta: {
    isReadPage: false,
    isViewPage: false,
    filterString: null,
    baseName: 'login',
    getParentPage: (route) => {
      return {
        name: 'home',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    },
    getChildPage: (route) => {
      return {
        name: 'login',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    }
  }
}
