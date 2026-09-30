<template>
  <n-layout has-sider style="height: 100vh">
    <!-- 侧边栏 -->
    <n-layout-sider
      collapse-mode="width"
      :collapsed-width="64"
      :width="240"
      :collapsed="collapsed"
      show-trigger="bar"
      @collapse="collapsed = true"
      @expand="collapsed = false"
      :native-scrollbar="false"
      class="app-sider"
    >
      <!-- Logo -->
      <div class="logo" :class="{ 'logo-collapsed': collapsed }">
        <div class="logo-mark">
          <n-icon :size="collapsed ? 24 : 28" color="#4A7BFF">
            <ShieldCheckmarkOutline />
          </n-icon>
        </div>
        <div v-if="!collapsed" class="logo-text">
          <span class="logo-title">IoT-ZTNA</span>
          <span class="logo-subtitle">零信任安全网关</span>
        </div>
      </div>

      <n-menu
        :value="activeKey"
        :collapsed="collapsed"
        :collapsed-width="64"
        :collapsed-icon-size="20"
        :icon-size="20"
        :options="menuOptions"
        @update:value="handleMenuSelect"
        :indent="20"
        :root-indent="18"
        class="app-menu"
        :theme-overrides="menuThemeOverrides"
      />
    </n-layout-sider>

    <!-- 主内容区 -->
    <n-layout>
      <!-- 顶部导航 -->
      <n-layout-header class="app-header">
        <div class="header-left">
          <n-icon size="20" color="#165DFF">
            <component :is="currentPageIcon" />
          </n-icon>
          <span class="header-title">{{ currentPageTitle }}</span>
        </div>
        <div class="header-right">
          <n-badge :value="unackAlerts" :max="99" :show="unackAlerts > 0">
            <n-button quaternary circle @click="$router.push('/alerts')">
              <template #icon>
                <n-icon size="20"><NotificationsOutline /></n-icon>
              </template>
            </n-button>
          </n-badge>
          <n-divider vertical />
          <n-tag type="success" size="small" round>
            <template #icon>
              <n-icon><EllipseOutline /></n-icon>
            </template>
            网关在线
          </n-tag>
          <n-divider vertical />
          <n-dropdown :options="userMenuOptions" @select="handleUserMenu">
            <div class="user-trigger">
              <n-avatar round size="small" style="background: #165DFF">{{ userInitial }}</n-avatar>
              <span class="user-name">{{ currentUser }}</span>
              <n-icon size="14" color="#86909C"><ChevronDownOutline /></n-icon>
            </div>
          </n-dropdown>
        </div>
      </n-layout-header>

      <!-- 页面内容 -->
      <n-layout-content class="app-content">
        <router-view v-slot="{ Component }">
          <transition name="page-fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </n-layout-content>

      <!-- 修改密码弹窗 -->
      <n-modal v-model:show="showChangePwd" preset="dialog" title="修改密码" :show-icon="false" style="width: 440px">
        <n-form ref="pwdFormRef" :model="pwdForm" :rules="pwdRules" label-placement="top">
          <n-form-item label="旧密码" path="old_password">
            <n-input v-model:value="pwdForm.old_password" type="password" placeholder="请输入当前密码" show-password-on="click" />
          </n-form-item>
          <n-form-item label="新密码" path="new_password">
            <n-input v-model:value="pwdForm.new_password" type="password" placeholder="至少 6 个字符" show-password-on="click" />
          </n-form-item>
          <n-form-item label="确认新密码" path="confirm_password">
            <n-input v-model:value="pwdForm.confirm_password" type="password" placeholder="再次输入新密码" show-password-on="click" />
          </n-form-item>
        </n-form>
        <template #action>
          <n-button @click="showChangePwd = false">取消</n-button>
          <n-button type="primary" :loading="changingPwd" @click="handleChangePassword">确认修改</n-button>
        </template>
      </n-modal>
    </n-layout>
  </n-layout>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, h, type Component } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { NIcon, useMessage, type MenuOption } from 'naive-ui'

const menuThemeOverrides = {
  itemColor: 'transparent',
  itemColorHover: 'rgba(255, 255, 255, 0.1)',
  itemColorActive: 'rgba(22, 93, 255, 0.25)',
  itemColorActiveHover: 'rgba(22, 93, 255, 0.3)',
  itemColorActiveCollapsed: 'rgba(22, 93, 255, 0.25)',
  itemTextColor: 'rgba(255, 255, 255, 0.65)',
  itemTextColorHover: '#ffffff',
  itemTextColorActive: '#ffffff',
  itemTextColorActiveHover: '#ffffff',
  itemTextColorChildActive: '#ffffff',
  itemTextColorChildActiveHover: '#ffffff',
  itemIconColor: 'rgba(255, 255, 255, 0.55)',
  itemIconColorHover: '#ffffff',
  itemIconColorActive: '#4A7BFF',
  itemIconColorActiveHover: '#4A7BFF',
  itemIconColorChildActive: '#4A7BFF',
  itemIconColorChildActiveHover: '#4A7BFF',
  borderRadius: '8px',
}
import {
  GridOutline,
  HardwareChipOutline,
  LockClosedOutline,
  ShieldCheckmarkOutline,
  NotificationsOutline,
  PulseOutline,
  DocumentTextOutline,
  SettingsOutline,
  EllipseOutline,
  ChevronDownOutline,
  LogOutOutline,
  PersonOutline,
  PeopleOutline,
  KeyOutline,
  ServerOutline,
  CubeOutline,
  CodeSlashOutline,
  CheckmarkDoneOutline,
} from '@vicons/ionicons5'
import type { DropdownOption } from 'naive-ui'

const route = useRoute()
const router = useRouter()
const message = useMessage()
const collapsed = ref(false)
const unackAlerts = ref(0)

const currentUser = ref(localStorage.getItem('ztna_user') || 'admin')
const userInitial = computed(() => currentUser.value.charAt(0).toUpperCase())

const userMenuOptions: DropdownOption[] = [
  { label: '修改密码', key: 'change-password', icon: () => h(NIcon, null, { default: () => h(KeyOutline) }) },
  { type: 'divider', key: 'divider' },
  { label: '退出登录', key: 'logout', icon: () => h(NIcon, null, { default: () => h(LogOutOutline) }) },
]

function handleUserMenu(key: string) {
  if (key === 'logout') {
    localStorage.removeItem('ztna_token')
    localStorage.removeItem('ztna_role')
    localStorage.removeItem('ztna_user')
    router.push('/login')
  } else if (key === 'change-password') {
    pwdForm.value = { old_password: '', new_password: '', confirm_password: '' }
    showChangePwd.value = true
  }
}

// === 修改密码 ===
const showChangePwd = ref(false)
const changingPwd = ref(false)
const pwdFormRef = ref<any>(null)
const pwdForm = ref({
  old_password: '',
  new_password: '',
  confirm_password: '',
})
const pwdRules = {
  old_password: [{ required: true, message: '请输入旧密码', trigger: 'blur' }],
  new_password: [
    { required: true, message: '请输入新密码', trigger: 'blur' },
    { min: 6, message: '密码至少 6 个字符', trigger: 'blur' },
  ],
  confirm_password: [
    { required: true, message: '请确认新密码', trigger: 'blur' },
    {
      validator: (_rule: any, value: string) => value === pwdForm.value.new_password,
      message: '两次输入的密码不一致',
      trigger: 'blur',
    },
  ],
}

async function handleChangePassword() {
  try {
    await pwdFormRef.value?.validate()
  } catch {
    return
  }
  changingPwd.value = true
  try {
    const { default: api } = await import('@/api')
    const user = localStorage.getItem('ztna_user') || 'admin'
    // 通过用户名查找用户 ID
    const res = await api.get<{ users: any[] }>('/users')
    if (res.success && res.data) {
      const userObj = res.data.users.find((u: any) => u.username === user)
      if (userObj) {
        await api.post(`/users/${userObj.id}/password`, {
          old_password: pwdForm.value.old_password,
          new_password: pwdForm.value.new_password,
        })
        message.success('密码修改成功')
        showChangePwd.value = false
      }
    }
  } catch (e: any) {
    message.error('修改失败: ' + (e.message || '未知错误'))
  } finally {
    changingPwd.value = false
  }
}

function renderIcon(icon: any) {
  return () => h(NIcon, null, { default: () => h(icon) })
}

const menuIcons: Record<string, Component> = {
  Dashboard: GridOutline,
  Devices: HardwareChipOutline,
  Certificates: LockClosedOutline,
  Policies: ShieldCheckmarkOutline,
  Alerts: NotificationsOutline,
  Behavior: PulseOutline,
  Audit: DocumentTextOutline,
  Settings: SettingsOutline,
  Users: PeopleOutline,
  Approvals: CheckmarkDoneOutline,
  Models: CubeOutline,
  Dataplane: ServerOutline,
  RegoPolicy: CodeSlashOutline,
}

const menuOptions: MenuOption[] = [
  { label: '仪表盘', key: 'Dashboard', icon: renderIcon(GridOutline) },
  { label: '设备管理', key: 'Devices', icon: renderIcon(HardwareChipOutline) },
  { label: '证书管理', key: 'Certificates', icon: renderIcon(LockClosedOutline) },
  { label: '策略管理', key: 'Policies', icon: renderIcon(ShieldCheckmarkOutline) },
  { label: 'Rego 引擎', key: 'RegoPolicy', icon: renderIcon(CodeSlashOutline) },
  { label: '策略审批', key: 'Approvals', icon: renderIcon(CheckmarkDoneOutline) },
  { label: '告警中心', key: 'Alerts', icon: renderIcon(NotificationsOutline) },
  { label: '行为检测', key: 'Behavior', icon: renderIcon(PulseOutline) },
  { label: '模型管理', key: 'Models', icon: renderIcon(CubeOutline) },
  { label: '数据面监控', key: 'Dataplane', icon: renderIcon(ServerOutline) },
  { label: '安全审计', key: 'Audit', icon: renderIcon(DocumentTextOutline) },
  { label: '用户管理', key: 'Users', icon: renderIcon(PeopleOutline) },
  { label: '系统设置', key: 'Settings', icon: renderIcon(SettingsOutline) },
]

const activeKey = computed(() => route.name as string)
const currentPageTitle = computed(() => {
  const option = menuOptions.find(o => o.key === activeKey.value)
  return option?.label as string || ''
})
const currentPageIcon = computed(() => {
  return menuIcons[activeKey.value as string] || GridOutline
})

function handleMenuSelect(key: string) {
  router.push({ name: key })
}

onMounted(async () => {
  try {
    const { default: api } = await import('@/api')
    await api.get('/health')
    // 获取未确认告警数
    const res = await api.get('/alerts?page=1&page_size=1')
    if (res.success && res.data) {
      unackAlerts.value = (res.data as any).total || 0
    }
  } catch {
    // ignore
  }
})
</script>

<style scoped>
/* === 侧边栏 === */
.app-sider {
  background: linear-gradient(180deg, #0C1322 0%, #0F1B33 100%) !important;
  box-shadow: 2px 0 12px rgba(0, 0, 0, 0.15);
  border-right: 1px solid rgba(255, 255, 255, 0.04);
}

/* Logo */
.logo {
  height: 60px;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 18px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  transition: padding 0.3s;
}
.logo-collapsed {
  justify-content: center;
  padding: 0;
}
.logo-mark {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  width: 40px;
  height: 40px;
  border-radius: 10px;
  background: rgba(74, 123, 255, 0.12);
}
.logo-text {
  display: flex;
  flex-direction: column;
  line-height: 1.3;
}
.logo-title {
  color: #fff;
  font-size: 1.15rem;
  font-weight: 700;
  letter-spacing: 0.5px;
}
.logo-subtitle {
  color: rgba(255, 255, 255, 0.4);
  font-size: 0.62rem;
  letter-spacing: 1.2px;
  text-transform: uppercase;
}

/* 菜单样式已在 main.css 全局覆盖，这里仅放 scoped layout 样式 */

/* === 顶部导航 === */
.app-header {
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  background: #fff;
  border-bottom: 1px solid #E5E6EB;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.03);
}
.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}
.header-title {
  font-size: 17px;
  font-weight: 600;
  color: #1D2129;
}
.header-right {
  display: flex;
  align-items: center;
  gap: 16px;
}
.user-trigger {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 8px;
  transition: background 0.2s;
}
.user-trigger:hover {
  background: #F2F3F5;
}
.user-name {
  font-size: 0.85rem;
  color: #1D2129;
  font-weight: 500;
}

/* === 内容区 === */
.app-content {
  padding: 24px;
  background: #F2F3F5;
  overflow-y: auto;
}

/* 页面切换动画 */
.page-fade-enter-active,
.page-fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}
.page-fade-enter-from {
  opacity: 0;
  transform: translateY(8px);
}
.page-fade-leave-to {
  opacity: 0;
}
</style>
