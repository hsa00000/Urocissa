// src/router.ts

import { createRouter, createWebHistory, RouteRecordRaw } from 'vue-router'
import 'vue-router'

import UniversalSharePage from '@/components/Page/UniversalSharePage.vue'
import ViewPageMain from '@/components/View/ViewPageMain.vue'
import { useBasicStringStore } from '@/store/basicStringStore'
import { useShareStore } from '@/store/shareStore'
import { tagsRoute } from './tagsRoute'
import { linksRoute } from './linksRoute'
import { loginRoute } from './loginRoute'
import { PageReturnType } from './pageReturnType'

// ======================================
// Helper Function to Create Virtual Routes
// ======================================

type VirtualRouteName = 'home' | 'all' | 'favorite' | 'archived' | 'trashed' | 'albums' | 'videos'

function createVirtualRoute(baseName: VirtualRouteName): RouteRecordRaw {
  return {
    path: `/${baseName}`,
    name: baseName,
    meta: {
      isReadPage: false,
      isViewPage: false,
      baseName: baseName,
      getParentPage: (route) => {
        return {
          name: baseName,
          params: { hash: undefined, subhash: undefined },
          query: route.query
        }
      },
      getChildPage: (route, hash) => {
        return {
          name: `${baseName}ViewPage`,
          params: { hash: hash, subhash: undefined },
          query: route.query
        }
      }
    },
    beforeEnter: (_to, _from, next) => {
      const basicStringStore = useBasicStringStore('mainId')

      let newBasicString: string
      switch (baseName) {
        case 'home':
          newBasicString = 'and(not(tag:"_archived"), not(tag:"_trashed"))'
          break
        case 'all':
          newBasicString = 'not(tag:"_trashed")'
          break
        case 'favorite':
          newBasicString = 'and(tag:"_favorite", not(tag:"_trashed"))'
          break
        case 'archived':
          newBasicString = 'and(tag:"_archived", not(tag:"_trashed"))'
          break
        case 'trashed':
          newBasicString = 'and(tag:"_trashed")'
          break
        case 'albums':
          newBasicString = 'and(type:"album", not(tag:"_trashed"))'
          break
        case 'videos':
          newBasicString = 'and(type:"video", not(tag:"_archived"), not(tag:"_trashed"))'
          break
      }

      basicStringStore.basicString = newBasicString
      next()
    },
    children: [
      {
        path: '/view/:hash',
        component: ViewPageMain,
        name: `${baseName}ViewPage`,
        meta: {
          isReadPage: false,
          isViewPage: true,
          baseName: baseName,
          getParentPage: (route) => {
            return {
              name: baseName,
              params: { hash: undefined, subhash: undefined },
              query: route.query
            }
          },
          getChildPage: () => {
            throw new Error('Read page temporarily unavailable')
          }
        }
      }
    ]
  }
}

// ======================================
// Define Simple Static Routes
// ======================================

const simpleRoutes: RouteRecordRaw[] = [
  { path: '/', redirect: '/home' },
  tagsRoute,
  linksRoute,
  loginRoute
]

// ======================================
// Define Share Route
// ======================================

const shareRoute: RouteRecordRaw = {
  path: '/share/:albumId-:shareId',
  component: UniversalSharePage,
  name: 'share',
  meta: {
    isReadPage: false,
    isViewPage: false,
    basicString: null,
    baseName: 'share',
    getParentPage: (route) => {
      return {
        name: 'share',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    },
    getChildPage: (route, hash) => {
      return {
        name: `shareViewPage`,
        params: { hash: hash, subhash: undefined },
        query: route.query
      }
    }
  },
  beforeEnter: (to, _from, next) => {
    const albumIdOpt = to.params.albumId
    const shareIdOpt = to.params.shareId

    // Only allow route if both albumId and shareId are strings
    if (typeof albumIdOpt === 'string' && typeof shareIdOpt === 'string') {
      // Set up the basic string and store data
      const basicStringStore = useBasicStringStore('mainId')
      const shareStore = useShareStore('mainId')

      basicStringStore.basicString = `and(not(tag:"_trashed"), album:"${albumIdOpt}")`
      shareStore.albumId = albumIdOpt
      shareStore.shareId = shareIdOpt

      next()
    } else {
      console.error(`(albumId, shareId) is (${String(albumIdOpt)}, ${String(shareIdOpt)})`)
      // Redirect to home page
      next('/home')
    }
  },
  children: [
    {
      path: 'view/:hash',
      component: ViewPageMain,
      name: `shareViewPage`,
      meta: {
        isReadPage: false,
        isViewPage: true,
        baseName: 'share',
        getParentPage: (route, albumId, shareId) => {
          return {
            name: 'share',
            params: { albumId: albumId, shareId: shareId },
            query: route.query
          }
        },
        getChildPage: function (): PageReturnType {
          throw new Error('Function not implemented.')
        }
      }
    }
  ]
}

// 定義個別的虛擬路由
const individualRoutes: RouteRecordRaw[] = [
  'home',
  'all',
  'favorite',
  'archived',
  'trashed',
  'albums',
  'videos'
].map((baseName) => createVirtualRoute(baseName as VirtualRouteName))

// ======================================
// Combine All Routes
// ======================================

const routes: RouteRecordRaw[] = [...simpleRoutes, ...individualRoutes, shareRoute]

// ======================================
// Create and Export the Router Instance
// ======================================

const router = createRouter({
  history: createWebHistory(),
  routes
})

export default router
