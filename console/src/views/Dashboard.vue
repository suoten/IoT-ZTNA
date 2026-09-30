<template>
  <div>
    <!-- 新手引导 -->
    <n-alert
      v-if="showGuide"
      type="info"
      closable
      title="快速上手指南"
      style="margin-bottom: 16px"
      @close="dismissGuide"
    >
      <n-space vertical :size="8">
        <span>欢迎使用 IoT-ZTNA 零信任网关！请按以下步骤完成初始化：</span>
        <n-steps :current="guideStep" size="small" :status="guideStep >= 3 ? 'finish' : 'process'">
          <n-step title="初始化 CA" description="在系统设置中生成根证书" />
          <n-step title="注册设备" description="在设备管理中添加 IoT 设备" />
          <n-step title="发布策略" description="在策略管理中创建并发布访问策略" />
        </n-steps>
        <n-space>
          <n-button size="small" type="primary" @click="$router.push('/settings')">前往设置</n-button>
          <n-button size="small" @click="$router.push('/devices')">注册设备</n-button>
          <n-button size="small" quaternary @click="$router.push('/policies')">配置策略</n-button>
        </n-space>
      </n-space>
    </n-alert>

    <!-- 统计卡片 -->
    <n-grid :cols="4" :x-gap="16" :y-gap="16" responsive="screen" item-responsive>
      <n-grid-item span="4 m:2 l:1">
        <div class="stat-card" style="--accent: #165DFF">
          <div class="stat-icon">
            <n-icon :size="24" color="#165DFF"><HardwareChipOutline /></n-icon>
          </div>
          <div class="stat-body">
            <div class="stat-value">{{ stats.devicesTotal }}</div>
            <div class="stat-label">设备总数</div>
          </div>
        </div>
      </n-grid-item>
      <n-grid-item span="4 m:2 l:1">
        <div class="stat-card" style="--accent: #18A058">
          <div class="stat-icon">
            <n-icon :size="24" color="#18A058"><PulseOutline /></n-icon>
          </div>
          <div class="stat-body">
            <div class="stat-value" style="color: #18A058">{{ stats.devicesOnline }}</div>
            <div class="stat-label">在线设备</div>
          </div>
        </div>
      </n-grid-item>
      <n-grid-item span="4 m:2 l:1">
        <div class="stat-card" style="--accent: #D03050">
          <div class="stat-icon">
            <n-icon :size="24" color="#D03050"><ShieldCheckmarkOutline /></n-icon>
          </div>
          <div class="stat-body">
            <div class="stat-value" style="color: #D03050">{{ stats.interceptions }}</div>
            <div class="stat-label">拦截次数</div>
          </div>
        </div>
      </n-grid-item>
      <n-grid-item span="4 m:2 l:1">
        <div class="stat-card" style="--accent: #F0A020">
          <div class="stat-icon">
            <n-icon :size="24" color="#F0A020"><AlertCircleOutline /></n-icon>
          </div>
          <div class="stat-body">
            <div class="stat-value" style="color: #F0A020">{{ stats.alertsUnack }}</div>
            <div class="stat-label">未确认告警</div>
          </div>
        </div>
      </n-grid-item>
    </n-grid>

    <!-- 图表区 -->
    <n-grid :cols="2" :x-gap="16" :y-gap="16" style="margin-top: 16px">
      <n-grid-item>
        <div class="card">
          <div class="card-header">
            <n-icon size="18" color="#165DFF"><PieChartOutline /></n-icon>
            <span class="card-title">设备状态分布</span>
          </div>
          <v-chart :option="deviceChartOption" style="height: 280px" autoresize />
        </div>
      </n-grid-item>
      <n-grid-item>
        <div class="card">
          <div class="card-header">
            <n-icon size="18" color="#165DFF"><BarChartOutline /></n-icon>
            <span class="card-title">信任等级分布</span>
          </div>
          <v-chart :option="levelChartOption" style="height: 280px" autoresize />
        </div>
      </n-grid-item>
    </n-grid>

    <!-- 实时告警 -->
    <div class="card" style="margin-top: 16px">
      <div class="card-header" style="margin-bottom: 12px">
        <n-icon size="18" color="#F0A020"><NotificationsOutline /></n-icon>
        <span class="card-title">最近告警</span>
        <n-button size="small" quaternary type="primary" style="margin-left: auto" @click="$router.push('/alerts')">
          查看全部
        </n-button>
      </div>
      <n-data-table :columns="alertColumns" :data="recentAlerts" :bordered="false" size="small" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, h } from 'vue'
import VChart from 'vue-echarts'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { PieChart } from 'echarts/charts'
import { TitleComponent, TooltipComponent, LegendComponent } from 'echarts/components'
import { NTag } from 'naive-ui'
import {
  HardwareChipOutline,
  PulseOutline,
  ShieldCheckmarkOutline,
  AlertCircleOutline,
  PieChartOutline,
  BarChartOutline,
  NotificationsOutline,
} from '@vicons/ionicons5'
import { alertApi, type Alert } from '@/api'

use([CanvasRenderer, PieChart, TitleComponent, TooltipComponent, LegendComponent])

const stats = ref({
  devicesTotal: 0,
  devicesOnline: 0,
  interceptions: 0,
  alertsUnack: 0,
})

// 新手引导
const showGuide = ref(localStorage.getItem('ztna_guide_dismissed') !== 'true')
const guideStep = ref(0) // 0=未初始化 CA, 1=已初始化 CA, 2=已注册设备, 3=已发布策略

function dismissGuide() {
  showGuide.value = false
  localStorage.setItem('ztna_guide_dismissed', 'true')
}

const recentAlerts = ref<Alert[]>([])
let ws: WebSocket | null = null
let metricsTimer: number | null = null

const deviceChartOption = computed(() => ({
  tooltip: { trigger: 'item' },
  legend: { bottom: 0, textStyle: { color: '#86909C' } },
  series: [
    {
      type: 'pie',
      radius: ['45%', '70%'],
      avoidLabelOverlap: false,
      label: { show: false },
      data: [
        { value: stats.value.devicesOnline, name: '在线', itemStyle: { color: '#18A058' } },
        { value: stats.value.devicesTotal - stats.value.devicesOnline, name: '离线', itemStyle: { color: '#E5E6EB' } },
      ],
    },
  ],
}))

const levelChartOption = ref({
  tooltip: { trigger: 'item' },
  legend: { bottom: 0, textStyle: { color: '#86909C' } },
  series: [
    {
      type: 'pie',
      radius: '60%',
      data: [
        { value: 0, name: 'A级', itemStyle: { color: '#165DFF' } },
        { value: 0, name: 'B级', itemStyle: { color: '#7B61FF' } },
        { value: 0, name: 'C级', itemStyle: { color: '#00B42A' } },
      ],
    },
  ],
})

const alertColumns = [
  { title: '类型', key: 'alert_type', width: 150 },
  {
    title: '级别',
    key: 'severity',
    width: 100,
    render: (row: Alert) => {
      const type = row.severity === 'critical' ? 'error' : row.severity === 'warning' ? 'warning' : 'info'
      const label = row.severity === 'critical' ? '严重' : row.severity === 'warning' ? '警告' : '信息'
      return h(NTag, { type, size: 'small' }, () => label)
    },
  },
  { title: '消息', key: 'message' },
  { title: '时间', key: 'created_at', width: 180, render: (row: Alert) => new Date(row.created_at).toLocaleString() },
]

async function loadStats() {
  try {
    // 直接获取 metrics（metrics 端点不需要认证）
    const metricsRes = await fetch('/api/v1/metrics')
    if (!metricsRes.ok) throw new Error(`metrics HTTP ${metricsRes.status}`)
    const metricsText = await metricsRes.text()
    const parseMetric = (name: string) => {
      const match = metricsText.match(new RegExp(`${name}\\s+(\\d+)`))
      return match ? parseInt(match[1]) : 0
    }
    stats.value.devicesTotal = parseMetric('ztna_devices_total')
    stats.value.devicesOnline = parseMetric('ztna_devices_online')
    stats.value.interceptions = parseMetric('ztna_interceptions_total')
    stats.value.alertsUnack = parseMetric('ztna_alerts_unacknowledged')
  } catch (e) {
    console.error('加载统计失败', e)
  }
}

async function loadAlerts() {
  try {
    const res = await alertApi.list(1, 5)
    if (res.success && res.data) {
      recentAlerts.value = res.data.items || []
    }
  } catch (e) {
    console.error('加载告警失败', e)
  }
}

function connectWs() {
  const proto = window.location.protocol === 'https:' ? 'wss' : 'ws'
  ws = new WebSocket(`${proto}://${window.location.host}/ws?channel=all`)
  ws.onmessage = (event) => {
    try {
      const msg = JSON.parse(event.data)
      if (msg.channel === 'metrics' && msg.data) {
        stats.value.devicesTotal = msg.data.devices_total || 0
        stats.value.devicesOnline = msg.data.devices_online || 0
        stats.value.alertsUnack = msg.data.alerts_unack || 0
      }
      if (msg.channel === 'alerts') {
        loadAlerts()
      }
    } catch {
      // ignore
    }
  }
  ws.onclose = () => {
    setTimeout(connectWs, 5000)
  }
}

onMounted(() => {
  loadStats()
  loadAlerts()
  connectWs()
  metricsTimer = window.setInterval(loadStats, 30000)
})

onUnmounted(() => {
  ws?.close()
  if (metricsTimer) clearInterval(metricsTimer)
})
</script>

<style scoped>
/* Dashboard 特有样式 - 通用卡片/统计样式已在 main.css 中定义 */
</style>
