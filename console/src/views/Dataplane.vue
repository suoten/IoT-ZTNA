<template>
  <div>
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px">
      <h2 class="page-title">数据面监控</h2>
      <n-space>
        <n-button @click="loadData" :loading="loading">
          <template #icon><n-icon><RefreshOutline /></n-icon></template>
          刷新
        </n-button>
        <n-button type="error" @click="handleClearAll" :loading="clearing">
          <template #icon><n-icon><TrashOutline /></n-icon></template>
          清除所有规则
        </n-button>
      </n-space>
    </div>

    <!-- 引导提示 -->
    <n-alert type="info" style="margin-bottom: 16px" :bordered="false">
      <template #header>
        <span style="font-size: 0.85rem">💡 数据面说明</span>
      </template>
      <div style="font-size: 0.8rem; color: #4E5969; line-height: 1.8">
        数据面是策略的执行层。在 Linux 环境（内核 ≥ 5.10）下使用 eBPF/XDP 内核态过滤，延迟 < 30μs。
        <br />
        不满足 eBPF 条件时自动降级到 nftables 内核态执行。Windows 开发环境为内存模式（仅统计）。
      </div>
    </n-alert>

    <n-grid :cols="3" :x-gap="16" :y-gap="16" v-if="stats">
      <n-gi>
        <n-card size="small">
          <n-statistic label="执行模式" :value="modeLabel(stats.mode)">
            <template #suffix>
              <n-tag :type="stats.is_ebpf ? 'success' : 'warning'" size="small" style="margin-left: 8px">
                {{ stats.is_ebpf ? 'eBPF/XDP' : 'nftables' }}
              </n-tag>
            </template>
          </n-statistic>
        </n-card>
      </n-gi>
      <n-gi>
        <n-card size="small">
          <n-statistic label="活跃规则数" :value="stats.active_rules" />
        </n-card>
      </n-gi>
      <n-gi>
        <n-card size="small">
          <n-statistic label="累计拦截包数" :value="stats.packets_dropped" />
        </n-card>
      </n-gi>
      <n-gi>
        <n-card size="small">
          <n-statistic label="累计通过包数" :value="stats.packets_passed" />
        </n-card>
      </n-gi>
      <n-gi>
        <n-card size="small">
          <n-statistic label="通过字节数" :value="stats.bytes_passed" />
        </n-card>
      </n-gi>
    </n-grid>

    <div class="card" style="margin-top: 16px">
      <n-card title="架构说明" size="small">
        <n-space vertical>
          <n-alert type="success" :bordered="false" v-if="stats?.is_ebpf">
            <strong>eBPF/XDP 数据面已激活</strong> — 包过滤在内核态执行，延迟 ~10-30μs，支持万兆线速。
          </n-alert>
          <n-alert type="warning" :bordered="false" v-else>
            <strong>nftables 降级模式</strong> — eBPF 不可用（需要 Linux 内核 ≥ 5.10 + root 权限），已降级到 nftables 内核态执行。
          </n-alert>
          <n-space>
            <n-tag type="info" size="small">XDP 程序</n-tag>
            <n-tag type="info" size="small">TC 程序</n-tag>
            <n-tag type="info" size="small">BPF Map 策略同步</n-tag>
            <n-tag type="info" size="small">nftables 降级</n-tag>
          </n-space>
        </n-space>
      </n-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NCard, NStatistic, NSpace, NAlert, NTag, NGrid, NGi, NIcon, useMessage } from 'naive-ui'
import { RefreshOutline, TrashOutline } from '@vicons/ionicons5'
import { dataplaneApi } from '@/api'

const message = useMessage()
const loading = ref(false)
const clearing = ref(false)
const stats = ref<any>(null)

function modeLabel(mode: string): string {
  const map: Record<string, string> = {
    'ebpf': 'eBPF/XDP',
    'nftables': 'nftables',
    'memory': '内存模式',
    'unknown': '未知',
  }
  return map[mode] || mode || '-'
}

async function loadData() {
  loading.value = true
  try {
    const res = await dataplaneApi.stats()
    if (res.success && res.data) stats.value = res.data
  } catch (e: any) {
    message.error(e.message)
  } finally {
    loading.value = false
  }
}

async function handleClearAll() {
  const confirmed = window.confirm('确定要清除所有数据面规则吗？')
  if (!confirmed) return
  clearing.value = true
  try {
    await dataplaneApi.clearAll()
    message.success('所有数据面规则已清除')
    loadData()
  } catch (e: any) {
    message.error(e.message)
  } finally {
    clearing.value = false
  }
}

onMounted(loadData)
</script>
