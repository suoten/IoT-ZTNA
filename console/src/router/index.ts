import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/Login.vue'),
    meta: { title: '登录', public: true },
  },
  {
    path: '/',
    component: () => import('@/layouts/MainLayout.vue'),
    redirect: '/dashboard',
    children: [
      {
        path: 'dashboard',
        name: 'Dashboard',
        component: () => import('@/views/Dashboard.vue'),
        meta: { title: '仪表盘' },
      },
      {
        path: 'devices',
        name: 'Devices',
        component: () => import('@/views/Devices.vue'),
        meta: { title: '设备管理' },
      },
      {
        path: 'certificates',
        name: 'Certificates',
        component: () => import('@/views/Certificates.vue'),
        meta: { title: '证书管理' },
      },
      {
        path: 'policies',
        name: 'Policies',
        component: () => import('@/views/Policies.vue'),
        meta: { title: '策略管理' },
      },
      {
        path: 'alerts',
        name: 'Alerts',
        component: () => import('@/views/Alerts.vue'),
        meta: { title: '告警中心' },
      },
      {
        path: 'behavior',
        name: 'Behavior',
        component: () => import('@/views/Behavior.vue'),
        meta: { title: '行为检测' },
      },
      {
        path: 'audit',
        name: 'Audit',
        component: () => import('@/views/Audit.vue'),
        meta: { title: '安全审计' },
      },
      {
        path: 'settings',
        name: 'Settings',
        component: () => import('@/views/Settings.vue'),
        meta: { title: '系统设置' },
      },
      {
        path: 'users',
        name: 'Users',
        component: () => import('@/views/Users.vue'),
        meta: { title: '用户管理' },
      },
      {
        path: 'approvals',
        name: 'Approvals',
        component: () => import('@/views/Approvals.vue'),
        meta: { title: '策略审批' },
      },
      {
        path: 'models',
        name: 'Models',
        component: () => import('@/views/Models.vue'),
        meta: { title: '模型管理' },
      },
      {
        path: 'dataplane',
        name: 'Dataplane',
        component: () => import('@/views/Dataplane.vue'),
        meta: { title: '数据面监控' },
      },
      {
        path: 'rego',
        name: 'RegoPolicy',
        component: () => import('@/views/RegoPolicy.vue'),
        meta: { title: 'Rego 策略引擎' },
      },
    ],
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

// 路由守卫：未登录跳转到登录页
router.beforeEach((to, _from, next) => {
  const token = localStorage.getItem('ztna_token')
  // 公开页面直接放行
  if (to.meta.public) {
    // 已登录时访问登录页，跳转首页
    if (token && to.name === 'Login') {
      next('/dashboard')
    } else {
      next()
    }
    return
  }
  // 未登录跳转登录页
  if (!token) {
    next('/login')
    return
  }
  next()
})

export default router
