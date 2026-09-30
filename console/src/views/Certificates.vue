<template>
  <div>
    <h2 class="page-title">证书管理</h2>

    <!-- 引导提示 -->
    <n-alert type="info" style="margin-bottom: 16px" :bordered="false">
      <template #header>
        <span style="font-size: 0.85rem">💡 证书说明</span>
      </template>
      <div style="font-size: 0.8rem; color: #4E5969; line-height: 1.8">
        每台设备注册后自动签发 SPIFFE X.509 证书，有效期 7 天，自动轮换续期。
        <br />
        可对证书进行续期或吊销操作。吊销后设备无法再接入网关。
      </div>
    </n-alert>

    <div class="card">
      <n-data-table
        :columns="columns"
        :data="certificates"
        :loading="loading"
        :pagination="pagination"
      >
        <template #empty>
          <n-empty description="还没有证书">
            <template #icon>
              <n-icon size="48" color="#86909C"><LockClosedOutline /></n-icon>
            </template>
            <template #extra>
              <n-text depth="3" style="font-size: 0.85rem">注册设备后，系统会自动签发证书</n-text>
            </template>
          </n-empty>
        </template>
      </n-data-table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, h } from 'vue'
import { NTag, NButton, NSpace, NIcon, NAlert, useMessage } from 'naive-ui'
import { RefreshOutline, BanOutline, LockClosedOutline } from '@vicons/ionicons5'
import { certApi, type Certificate } from '@/api'

const message = useMessage()
const loading = ref(false)
const certificates = ref<Certificate[]>([])
const pagination = ref({ page: 1, pageSize: 20 })

const columns = [
  { title: 'SPIFFE ID', key: 'spiffe_id', ellipsis: { tooltip: true } },
  { title: '序列号', key: 'serial_number', width: 280 },
  {
    title: '状态',
    key: 'status',
    width: 100,
    render: (row: Certificate) => {
      const type = row.status === 'active' ? 'success' : row.status === 'revoked' ? 'error' : 'default'
      const label = row.status === 'active' ? '有效' : row.status === 'revoked' ? '已吊销' : row.status === 'expired' ? '已过期' : row.status
      return h(NTag, { type, size: 'small' }, () => label)
    },
  },
  {
    title: '签发时间',
    key: 'issued_at',
    width: 180,
    render: (row: Certificate) => new Date(row.issued_at).toLocaleString(),
  },
  {
    title: '到期时间',
    key: 'expires_at',
    width: 180,
    render: (row: Certificate) => new Date(row.expires_at).toLocaleString(),
  },
  {
    title: '剩余天数',
    key: 'remaining',
    width: 100,
    render: (row: Certificate) => {
      const days = Math.ceil((new Date(row.expires_at).getTime() - Date.now()) / 86400000)
      const color = days < 0 ? '#d03050' : days < 7 ? '#f0a020' : '#18a058'
      return h('span', { style: { color, fontWeight: 600 } }, days < 0 ? '已过期' : `${days}天`)
    },
  },
  {
    title: '操作',
    key: 'actions',
    width: 150,
    render: (row: Certificate) =>
      h(NSpace, {}, () => [
        h(NButton, {
          size: 'small',
          type: 'info',
          disabled: row.status !== 'active',
          onClick: () => handleRenew(row),
        }, () => '续期'),
        h(NButton, {
          size: 'small',
          type: 'error',
          disabled: row.status !== 'active',
          onClick: () => handleRevoke(row),
        }, () => '吊销'),
      ]),
  },
]

async function loadData() {
  loading.value = true
  try {
    const res = await certApi.list(1, 100)
    if (res.success && res.data) {
      certificates.value = res.data.items || []
    }
  } catch (e: any) {
    message.error(e.message)
  } finally {
    loading.value = false
  }
}

async function handleRenew(row: Certificate) {
  try {
    await certApi.renew(row.id)
    message.success('证书续期成功')
    loadData()
  } catch (e: any) {
    message.error(e.message)
  }
}

async function handleRevoke(row: Certificate) {
  const confirmed = window.confirm(`确定要吊销证书 ${row.serial_number.substring(0, 8)}... 吗？此操作不可逆。`)
  if (!confirmed) return
  try {
    await certApi.revoke(row.id)
    message.success('证书已吊销')
    loadData()
  } catch (e: any) {
    message.error(e.message)
  }
}

onMounted(loadData)
</script>
