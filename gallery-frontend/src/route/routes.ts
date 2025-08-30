import { createRouter, createWebHistory, RouteRecordRaw } from 'vue-router'
import type { RouteLocationRaw } from 'vue-router'
import 'vue-router'

import { createRoute } from './createRoute'
import { tagsRoute } from './tagsRoute'
import { linksRoute } from './linksRoute'
import { loginRoute } from './loginRoute'
import { shareRoute } from './shareRoute'

const simpleRoutes: RouteRecordRaw[] = [
  { path: '/', redirect: '/home' },
  tagsRoute,
  linksRoute,
  loginRoute,
  shareRoute
]

const homePageRoutes = createRoute('home', () => import('@/components/Page/HomePage.vue'))

const allPageRoutes = createRoute('all', () => import('@/components/Page/AllPage.vue'))

const favoritePageRoutes = createRoute(
  'favorite',
  () => import('@/components/Page/FavoritePage.vue')
)

const archivedPageRoutes = createRoute(
  'archived',
  () => import('@/components/Page/ArchivedPage.vue')
)

const trashedPageRoutes = createRoute('trashed', () => import('@/components/Page/TrashedPage.vue'))

const albumsPageRoutes = createRoute('albums', () => import('@/components/Page/AlbumsPage.vue'))

const videosPageRoutes = createRoute('videos', () => import('@/components/Page/VideosPage.vue'))

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

const router = createRouter({
  history: createWebHistory(),
  routes
})

router.afterEach((to) => {
  const baseName =
    typeof to.meta.baseName === 'string'
      ? to.meta.baseName
      : typeof to.name === 'string'
      ? to.name
      : undefined

  const baseTitleMap: Record<string, string> = {
    home: 'Home',
    all: 'All',
    favorite: 'Favorites',
    archived: 'Archived',
    trashed: 'Trash',
    albums: 'Albums',
    videos: 'Videos',
    album: 'Album',
    tags: 'Tags',
    links: 'Links',
    login: 'Login',
    share: 'Share'
  }

  let baseTitle: string
  if (baseName != null && baseName !== '') {
    baseTitle = baseTitleMap[baseName] ?? baseName
  } else {
    baseTitle = typeof to.name === 'string' ? to.name : 'Urocissa'
  }
  const isView = typeof to.meta.isViewPage === 'boolean' ? to.meta.isViewPage : false
  const isRead = typeof to.meta.isReadPage === 'boolean' ? to.meta.isReadPage : false

  // When on a View page, append the hash (or subhash for read-view) to the title
  let suffix = ''
  if (isView) {
    const maybeHash =
      typeof to.params.subhash === 'string'
        ? to.params.subhash
        : typeof to.params.hash === 'string'
        ? to.params.hash
        : undefined
    if (maybeHash != null && maybeHash !== '') {
      suffix = `View ${maybeHash}`
    } else {
      suffix = 'View'
    }
  } else if (isRead) {
    suffix = 'Reader'
  }

  const pageTitle = suffix ? `${baseTitle} ${suffix}` : baseTitle

  document.title = `${pageTitle} - Urocissa`
})

// On first app load, if user lands directly on a nested page (view/read),
// synthesize the parent entry in history so a simple router.back() returns to it.
void router.isReady().then(async () => {
  const to = router.currentRoute.value
  const meta = to.meta

  // Only act on initial load for nested pages (read/view)
  const isNested = meta.level > 1
  if (isNested) {
    const routeName = to.name
    const baseName = meta.baseName

    if (typeof routeName !== 'string' || typeof baseName !== 'string') {
      return
    }

    const q = to.query
    const hashParam = typeof to.params.hash === 'string' ? to.params.hash : undefined
    // subhash is not needed for ancestors, only for target which we restore via fullPath

    const chain: RouteLocationRaw[] = []
    // Always build from top-most parent to immediate parent to ensure multi-step back works
    if (routeName === `${baseName}ReadViewPage`) {
      if (hashParam === undefined) return
      chain.push({ name: baseName, query: q })
      chain.push({ name: `${baseName}ViewPage`, params: { hash: hashParam }, query: q })
      chain.push({ name: `${baseName}ReadPage`, params: { hash: hashParam }, query: q })
    } else if (routeName === `${baseName}ReadPage`) {
      if (hashParam === undefined) return
      chain.push({ name: baseName, query: q })
      chain.push({ name: `${baseName}ViewPage`, params: { hash: hashParam }, query: q })
    } else if (routeName === `${baseName}ViewPage`) {
      chain.push({ name: baseName, query: q })
    }

    if (chain.length > 0) {
      const target: RouteLocationRaw = { path: to.fullPath }
      try {
        // Replace current entry with the first ancestor, then push remaining ancestors, then restore target
        const first = chain[0]
        if (first !== undefined) {
          await router.replace(first)
          for (let i = 1; i < chain.length; i++) {
            const step = chain[i]
            if (step !== undefined) {
              await router.push(step)
            }
          }
        }
        await router.push(target)
      } catch {
        // No-op on navigation aborts
      }
    }
  }
})

export default router
