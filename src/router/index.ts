import type { RouteRecordRaw } from 'vue-router'

/** Segoe Fluent Icons 字形（WinUIonWeb 的 FontIcon 使用 SEGOEICONS.TTF） */
export const ICONS = {
  home: '\uE80F',
  source: '\uE8B7',
  shield: '\uEA18',
  patch: '\uE90F',
  tool: '\uE90F',
  config: '\uE713',
  build: '\uE950',
  cloud: '\uE753',
  settings: '\uE713',
  rocket: '\uE945',
}

export const routes: RouteRecordRaw[] = [
  { path: '/', redirect: '/dashboard' },
  {
    path: '/dashboard',
    name: 'dashboard',
    component: () => import('@/views/DashboardView.vue'),
    meta: { icon: ICONS.home, title: '概览' },
  },
  {
    path: '/source',
    name: 'source',
    component: () => import('@/views/SourceView.vue'),
    meta: { icon: ICONS.source, title: '内核源码' },
  },
  {
    path: '/toolchain',
    name: 'toolchain',
    component: () => import('@/views/ToolchainView.vue'),
    meta: { icon: ICONS.tool, title: '工具链' },
  },
  {
    path: '/susfs',
    name: 'susfs',
    component: () => import('@/views/SusfsView.vue'),
    meta: { icon: ICONS.patch, title: 'SUSFS 补丁' },
  },
  {
    path: '/ksu',
    name: 'ksu',
    component: () => import('@/views/KsuView.vue'),
    meta: { icon: ICONS.shield, title: 'KernelSU' },
  },
  {
    path: '/defconfig',
    name: 'defconfig',
    component: () => import('@/views/DefconfigView.vue'),
    meta: { icon: ICONS.config, title: 'defconfig' },
  },
  {
    path: '/build',
    name: 'build',
    component: () => import('@/views/BuildView.vue'),
    meta: { icon: ICONS.build, title: '构建内核' },
  },
  {
    path: '/actions',
    name: 'actions',
    component: () => import('@/views/ActionsView.vue'),
    meta: { icon: ICONS.cloud, title: '远程编译' },
  },
  {
    path: '/settings',
    name: 'settings',
    component: () => import('@/views/SettingsView.vue'),
    meta: { icon: ICONS.settings, title: '设置' },
  },
  { path: '/:pathMatch(.*)*', redirect: '/dashboard' },
]

/** NavigationView 的菜单数据源（MainMenuItems） */
export const navMenuItems = routes
  .filter((r) => r.path !== '/' && !r.path.includes(':pathMatch'))
  .map((r) => ({
    Tag: String(r.name),
    Icon: (r.meta?.icon as string) ?? '',
    Content: (r.meta?.title as string) ?? String(r.name),
  }))
