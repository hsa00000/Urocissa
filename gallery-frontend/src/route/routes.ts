// src/router.ts

import { createRouter, createWebHistory, RouteRecordRaw } from 'vue-router'
import 'vue-router'

import UniversalPage from '@/components/Page/UniversalPage.vue'
import UniversalSharePage from '@/components/Page/UniversalSharePage.vue'
import ViewPageMain from '@/components/View/ViewPageMain.vue'
import HomeIsolated from '@/components/Home/HomeIsolated.vue'
import ViewPageIsolated from '@/components/View/ViewPageIsolated.vue'
import { useBasicStringStore } from '@/store/basicStringStore'
import { useShareStore } from '@/store/shareStore'
import { tagsRoute } from './tagsRoute'
import { linksRoute } from './linksRoute'
import { loginRoute } from './loginRoute'
import { PageReturnType } from './pageReturnType'

// ======================================
// Define Simple Static Routes
// ======================================

const simpleRoutes: RouteRecordRaw[] = [
  { path: '/', redirect: '/gallery?type=home' },
  tagsRoute,
  linksRoute,
  loginRoute
]

// ======================================
// Define Universal Gallery Route
// ======================================

const galleryRoute: RouteRecordRaw = {
  path: '/gallery',
  component: UniversalPage,
  name: 'gallery',
  meta: {
    isReadPage: false,
    isViewPage: false,
    baseName: 'gallery',
    getParentPage: (route) => {
      return {
        name: 'gallery',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    },
    getChildPage: (route, hash) => {
      return {
        name: 'galleryViewPage',
        params: { hash: hash, subhash: undefined },
        query: route.query
      }
    }
  },
  children: [
    {
      path: 'view/:hash',
      component: ViewPageMain,
      name: 'galleryViewPage',
      meta: {
        isReadPage: false,
        isViewPage: true,
        baseName: 'gallery',
        getParentPage: (route) => {
          return {
            name: 'gallery',
            params: { hash: undefined, subhash: undefined },
            query: route.query
          }
        },
        getChildPage: (route) => {
          return {
            name: 'galleryReadPage',
            params: { hash: route.params.hash, subhash: undefined },
            query: route.query
          }
        }
      },
      children: [
        {
          path: 'read',
          component: HomeIsolated,
          name: 'galleryReadPage',
          meta: {
            isReadPage: true,
            isViewPage: false,
            baseName: 'gallery',
            getParentPage: (route) => {
              return {
                name: 'galleryViewPage',
                params: { hash: route.params.hash, subhash: undefined },
                query: route.query
              }
            },
            getChildPage: (route, subhash) => {
              return {
                name: 'galleryReadViewPage',
                params: { hash: route.params.hash, subhash: subhash },
                query: route.query
              }
            }
          },
          children: [
            {
              path: 'view/:subhash',
              name: 'galleryReadViewPage',
              component: ViewPageIsolated,
              meta: {
                isReadPage: true,
                isViewPage: true,
                baseName: 'gallery',
                getParentPage: (route) => {
                  return {
                    name: 'galleryReadPage',
                    params: { hash: route.params.hash, subhash: undefined },
                    query: route.query
                  }
                },
                getChildPage: (route) => {
                  return {
                    name: 'galleryReadViewPage',
                    params: { hash: route.params.hash, subhash: route.params.subhash },
                    query: route.query
                  }
                }
              }
            }
          ]
        }
      ]
    }
  ]
}

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
      // Redirect to gallery or show error page
      next('/gallery?type=home')
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

// 为了兼容性，添加重定向路由
const compatibilityRoutes: RouteRecordRaw[] = [
  { path: '/home', redirect: (to) => ({ path: '/gallery', query: { ...to.query, type: 'home' } }) },
  { path: '/all', redirect: (to) => ({ path: '/gallery', query: { ...to.query, type: 'all' } }) },
  {
    path: '/favorite',
    redirect: (to) => ({ path: '/gallery', query: { ...to.query, type: 'favorite' } })
  },
  {
    path: '/archived',
    redirect: (to) => ({ path: '/gallery', query: { ...to.query, type: 'archived' } })
  },
  {
    path: '/trashed',
    redirect: (to) => ({ path: '/gallery', query: { ...to.query, type: 'trashed' } })
  },
  {
    path: '/albums',
    redirect: (to) => ({ path: '/gallery', query: { ...to.query, type: 'albums' } })
  },
  {
    path: '/videos',
    redirect: (to) => ({ path: '/gallery', query: { ...to.query, type: 'videos' } })
  }
]

// ======================================
// Combine All Routes
// ======================================

const routes: RouteRecordRaw[] = [...simpleRoutes, galleryRoute, shareRoute, ...compatibilityRoutes]

// ======================================
// Create and Export the Router Instance
// ======================================

const router = createRouter({
  history: createWebHistory(),
  routes
})

export default router
