<template>
  <n-config-provider :theme="null" :theme-overrides="themeOverrides">
    <n-global-style />
    <n-message-provider>
      <n-dialog-provider>
        <n-notification-provider>
          <router-view />
        </n-notification-provider>
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import type { GlobalThemeOverrides } from 'naive-ui'

const themeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#165DFF',
    primaryColorHover: '#4080FF',
    primaryColorPressed: '#0E42D2',
    primaryColorSuppl: '#4080FF',
    borderRadius: '8px',
    fontFamily: "'Inter', 'Segoe UI', 'Microsoft YaHei', sans-serif",
  },
  Card: {
    borderRadius: '12px',
    boxShadow: '0 2px 12px rgba(0, 0, 0, 0.06)',
  },
  Button: {
    borderRadiusMedium: '8px',
  },
}
</script>
