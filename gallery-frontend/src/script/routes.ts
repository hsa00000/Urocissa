// src/router.ts

import { Component } from 'vue'
import { createRouter, createWebHistory, LocationQuery, RouteRecordRaw } from 'vue-router'
import 'vue-router'

import TagsPage from '@/components/Page/TagsPage.vue'
import LinksPage from '@/components/Page/LinksPage.vue'
import LoginPage from '@/components/LoginPage.vue'
import HomeMain from '@/components/Page/HomePage.vue'
import AllPage from '@/components/Page/AllPage.vue'
import FavoritePage from '@/components/Page/FavoritePage.vue'
import ArchivedPage from '@/components/Page/ArchivedPage.vue'
import TrashedPage from '@/components/Page/TrashedPage.vue'
import AlbumsPage from '@/components/Page/AlbumsPage.vue'
import ViewPageMain from '@/components/Home/View/Page/ViewPageMain.vue'
import HomeIsolated from '@/components/Home/Page/HomeIsolated.vue'
import ViewPageIsolated from '@/components/Home/View/Page/ViewPageIsolated.vue'
import VideosPage from '@/components/Page/VideosPage.vue'
import HomeShare from '@/components/Home/Page/HomeShare.vue'

interface PageReturnType {
  name: string
  params: {
    hash?: string | string[] | undefined
    subhash?: string | string[] | undefined
    albumId?: string | string[]
    shareId?: string | string[]
  }

  query: LocationQuery
}

type BaseName =
  | 'home'
  | 'all'
  | 'favorite'
  | 'archived'
  | 'trashed'
  | 'albums'
  | 'videos'
  | 'album'
  | 'tags'
  | 'login'
  | 'share'
  | 'links'

// ======================================
// 1. Define Simple Static Routes
// ======================================

const simpleRoutes: RouteRecordRaw[] = [
  { path: '/', redirect: '/home' },
  {
    path: '/tags',
    component: TagsPage,
    name: 'tags',
    meta: {
      isReadPage: false,
      isViewPage: false,
      basicString: null,
      baseName: 'tags',
      getParentPage: (route) => {
        return {
          name: 'home',
          params: { hash: undefined, subhash: undefined },
          query: route.query
        }
      },
      getChildPage: (route) => {
        return {
          name: 'tags',
          params: { hash: undefined, subhash: undefined },
          query: route.query
        }
      }
    }
  },
  {
    path: '/links',
    component: LinksPage,
    name: 'links',
    meta: {
      isReadPage: false,
      isViewPage: false,
      basicString: null,
      baseName: 'links',
      getParentPage: (route) => {
        return {
          name: 'home',
          params: { hash: undefined, subhash: undefined },
          query: route.query
        }
      },
      getChildPage: (route) => {
        return {
          name: 'links',
          params: { hash: undefined, subhash: undefined },
          query: route.query
        }
      }
    }
  },
  {
    path: '/login',
    component: LoginPage,
    name: 'login',
    meta: {
      isReadPage: false,
      isViewPage: false,
      basicString: null,
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
  },
  {
    path: '/share/:albumId-:shareId',
    component: HomeShare,
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
            console.log('123')
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
]

// ======================================
// 2. Define a Helper Function to Create Routes
// ======================================

/**
 * Creates a main route with an optional child route.
 *
 * @param path - The base path for the route.
 * @param component - The component to be rendered.
 * @param name - The unique name for the route.
 * @returns An array containing the RouteRecordRaw object.
 */
function createRoute(baseName: BaseName, component: Component): RouteRecordRaw[] {
  const mainRoute: RouteRecordRaw = {
    path: `/${baseName}`,
    component: component,
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
    children: [
      {
        path: 'view/:hash',
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
          getChildPage: (route) => {
            return {
              name: `${baseName}ReadPage`,
              params: { hash: route.params.hash, subhash: undefined },
              query: route.query
            }
          }
        },
        children: [
          {
            path: 'read',
            component: HomeIsolated,
            name: `${baseName}ReadPage`,
            meta: {
              isReadPage: true,
              isViewPage: false,
              baseName: baseName,
              getParentPage: (route) => {
                return {
                  name: `${baseName}ViewPage`,
                  params: { hash: route.params.hash, subhash: undefined },
                  query: route.query
                }
              },
              getChildPage: (route, subhash) => {
                return {
                  name: `${baseName}ReadViewPage`,
                  params: { hash: route.params.hash, subhash: subhash },
                  query: route.query
                }
              }
            },
            children: [
              {
                path: 'view/:subhash',
                name: `${baseName}ReadViewPage`,
                component: ViewPageIsolated,
                meta: {
                  isReadPage: true,
                  isViewPage: true,
                  baseName: baseName,
                  getParentPage: (route) => {
                    return {
                      name: `${baseName}ReadPage`,
                      params: { hash: route.params.hash, subhash: undefined },
                      query: route.query
                    }
                  },
                  getChildPage: (route) => {
                    return {
                      name: `${baseName}ReadViewPage`,
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
  return [mainRoute]
}

// ======================================
// 3. Create Routes Using the Helper Function
// ======================================

const homePageRoutes = createRoute('home', HomeMain)

const allPageRoutes = createRoute('all', AllPage)

const favoritePageRoutes = createRoute('favorite', FavoritePage)

const archivedPageRoutes = createRoute('archived', ArchivedPage)

const trashedPageRoutes = createRoute('trashed', TrashedPage)

const albumsPageRoutes = createRoute('albums', AlbumsPage)

const videosPageRoutes = createRoute('videos', VideosPage)

// ======================================
// 4. Combine All Routes
// ======================================

const routes: RouteRecordRaw[] = [
  ...simpleRoutes,
  ...homePageRoutes,
  ...allPageRoutes,
  ...favoritePageRoutes,
  ...archivedPageRoutes,
  ...trashedPageRoutes,
  ...albumsPageRoutes,
  ...videosPageRoutes
]

// ======================================
// 5. Create and Export the Router Instance
// ======================================

const router = createRouter({
  history: createWebHistory(),
  routes
})

declare module 'vue-router' {
  interface RouteMeta {
    isReadPage: boolean
    isViewPage: boolean
    baseName: BaseName
    getParentPage: (
      router: RouteLocationNormalizedLoadedGeneric,
      albumId?: string,
      shareId?: string
    ) => PageReturnType
    getChildPage: (
      router: RouteLocationNormalizedLoadedGeneric,
      hash: string | undefined
    ) => PageReturnType
  }
}

export default router
