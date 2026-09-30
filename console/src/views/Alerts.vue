<template>
  <div>
    <h2 class="page-title">告警中心</h2>
    <div class="card">
      <n-data-table :columns="columns" :data="alerts" :loading="loading" :pagination="pagination">
        <template #empty>
          <n-empty description="暂无告警">
            <template #icon>
              <n-icon size="48" color="#18A058"><CheckmarkCircleOutline /></n-icon>
            </template>
            <template #extra>
              <n-text depth="3" style="font-size: 0.85rem">系统正常运行中，无告警事件</n-text>
            </template>
          </n-empty>
        </template>
      </n-data-table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, h } from 'vue'
import { NTag, NButton, NSpace, NIcon, useMessage } from 'naive-ui'
import { CheckmarkCircleOutline } from '@vicons/ionicons5'
import { alertApi, type Alert } from '@/api'

const message = useMessage()
const loading = ref(false)
const alerts = ref<Alert[]>([])
const pagination = ref({ page: 1, pageSize: 20 })

const columns = [
  { title: '类型', key: 'alert_type', width: 180 },
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
  { title: 'SPIFFE ID', key: 'spiffe_id', ellipsis: { tooltip: true }, width: 300 },
  { title: '消息', key: 'message' },
  {
    title: '时间',
    key: 'created_at',
    width: 180,
    render: (row: Alert) => new Date(row.created_at).toLocaleString(),
  },
  {
    title: '状态',
    key: 'acknowledged',
    width: 100,
    render: (row: Alert) =>
      row.acknowledged ? h(NTag, { type: 'success', size: 'small' }, () => '已确认') : h(NTag, { type: 'warning', size: 'small' }, () => '未确认'),
  },
  {
    title: '操作',
    key: 'actions',
    width: 80,
    render: (row: Alert) =>
      h(NButton, {
        size: 'small',
        disabled: row.acknowledged,
        onClick: () => handleAck(row),
      }, () => '确认'),
  },
]

async function loadData() {
  loading.value = true
  try {
    const res = await alertApi.list(1, 100)
    if (res.success && res.data) {
      alerts.value = res.data.items || []
    }
  } catch (e: any) {
    message.error(e.message)
  } finally {
    loading.value = false
  }
}

async function handleAck(row: Alert) {
  try {
    await alertApi.ack(row.id)
    message.success('告警已确认')
    loadData()
  } catch (e: any) {
    message.error(e.message)
  }
}

onMounted(loadData)
</script>
