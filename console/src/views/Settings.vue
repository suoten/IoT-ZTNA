<template>
  <div>
    <h2 class="page-title">系统设置</h2>
    <n-grid :cols="2" :x-gap="16" :y-gap="16">
      <n-grid-item>
        <div class="card">
          <div class="card-header" style="margin-bottom: 16px">
            <n-icon size="18" color="#165DFF"><GlobeOutline /></n-icon>
            <span class="card-title">信任域配置</span>
          </div>
          <n-form label-placement="left" label-width="120">
            <n-form-item label="信任域"><n-input value="iot-ztna.local" disabled /></n-form-item>
            <n-form-item label="网关 ID"><n-input value="edge-gw-01" disabled /></n-form-item>
            <n-form-item label="SVID 有效期"><n-input value="7 天" disabled /></n-form-item>
            <n-form-item label="续期触发"><n-input value="到期前 24h" disabled /></n-form-item>
          </n-form>
        </div>
      </n-grid-item>
      <n-grid-item>
        <div class="card">
          <div class="card-header" style="margin-bottom: 16px">
            <n-icon size="18" color="#165DFF"><LockClosedOutline /></n-icon>
            <span class="card-title">CA 管理</span>
          </div>
          <n-space vertical>
            <n-button type="primary" block @click="handleRegenCa">
              <template #icon><n-icon><RefreshOutline /></n-icon></template>
              重新生成 Root CA
            </n-button>
            <n-button block @click="handleDownloadCa">
              <template #icon><n-icon><DownloadOutline /></n-icon></template>
              下载 CA 证书
            </n-button>
          </n-space>
        </div>
      </n-grid-item>
      <n-grid-item>
        <div class="card">
          <div class="card-header" style="margin-bottom: 16px">
            <n-icon size="18" color="#165DFF"><ServerOutline /></n-icon>
            <span class="card-title">流量执行引擎</span>
          </div>
          <n-form label-placement="left" label-width="120">
            <n-form-item label="表名"><n-input value="iot_ztna" disabled /></n-form-item>
            <n-form-item label="状态">
              <n-tag type="success" size="small">已启用</n-tag>
            </n-form-item>
            <n-form-item label="模式">
              <n-tag type="info" size="small">nftables / 内存降级</n-tag>
            </n-form-item>
            <n-form-item label="MQTT 代理">
              <n-tag type="success" size="small">端口 8883</n-tag>
            </n-form-item>
          </n-form>
        </div>
      </n-grid-item>
      <n-grid-item>
        <div class="card">
          <div class="card-header" style="margin-bottom: 16px">
            <n-icon size="18" color="#165DFF"><ShieldCheckmarkOutline /></n-icon>
            <span class="card-title">证书吊销 (CRL)</span>
          </div>
          <n-space vertical>
            <n-button block @click="handleViewCrl" :loading="loadingCrl">
              <template #icon><n-icon><ListOutline /></n-icon></template>
              查看吊销列表
            </n-button>
            <n-button block @click="handleCheckRevocation">
              <template #icon><n-icon><SearchOutline /></n-icon></template>
              检查证书吊销状态
            </n-button>
          </n-space>
        </div>
      </n-grid-item>
      <n-grid-item>
        <div class="card">
          <div class="card-header" style="margin-bottom: 16px">
            <n-icon size="18" color="#165DFF"><InformationCircleOutline /></n-icon>
            <span class="card-title">系统信息</span>
          </div>
          <n-descriptions :column="1" label-placement="left">
            <n-descriptions-item label="版本">v1.0.0</n-descriptions-item>
            <n-descriptions-item label="许可证">Apache-2.0</n-descriptions-item>
            <n-descriptions-item label="技术栈">Rust (Axum) + Vue3</n-descriptions-item>
            <n-descriptions-item label="标准">NIST SP 800-207 / 等保 2.0</n-descriptions-item>
            <n-descriptions-item label="数据库">SQLite (WAL)</n-descriptions-item>
            <n-descriptions-item label="认证">JWT + RBAC</n-descriptions-item>
          </n-descriptions>
        </div>
      </n-grid-item>
      <n-grid-item>
        <div class="card">
          <div class="card-header" style="margin-bottom: 16px">
            <n-icon size="18" color="#165DFF"><CloudDownloadOutline /></n-icon>
            <span class="card-title">集成对接</span>
          </div>
          <n-descriptions :column="1" label-placement="left">
            <n-descriptions-item label="REST API">
              <n-tag type="success" size="small">/api/v1</n-tag>
            </n-descriptions-item>
            <n-descriptions-item label="WebSocket">
              <n-tag type="success" size="small">/ws</n-tag>
            </n-descriptions-item>
            <n-descriptions-item label="Prometheus">
              <n-tag type="success" size="small">/api/v1/metrics</n-tag>
            </n-descriptions-item>
            <n-descriptions-item label="Webhook">
              <n-tag type="success" size="small">/api/v1/integrations</n-tag>
            </n-descriptions-item>
          </n-descriptions>
        </div>
      </n-grid-item>
    </n-grid>

    <!-- 数据库备份管理 -->
    <div class="card" style="margin-top: 16px">
      <div class="card-header" style="margin-bottom: 16px">
        <n-icon size="18" color="#165DFF"><CloudUploadOutline /></n-icon>
        <span class="card-title">数据库备份管理</span>
      </div>
      <n-space vertical>
        <!-- 引导提示 -->
        <n-alert type="info" :bordered="false">
          点击「备份数据库」可创建快照备份。备份文件保存为 SQLite 数据库副本，包含设备、策略、审计日志等全部数据。点击「下载」可将备份文件保存到本地。
        </n-alert>
        <n-space>
          <n-button type="primary" @click="handleBackup" :loading="backing">
            <template #icon><n-icon><CloudUploadOutline /></n-icon></template>
            备份数据库
          </n-button>
          <n-button @click="loadBackups" :loading="loadingBackups">
            <template #icon><n-icon><RefreshOutline /></n-icon></template>
            刷新列表
          </n-button>
        </n-space>
        <!-- 备份记录列表 -->
        <n-data-table
          v-if="backups.length > 0"
          :columns="backupColumns"
          :data="backups"
          :bordered="false"
          size="small"
        />
        <n-empty v-else description="暂无备份记录" style="padding: 24px">
          <template #icon>
            <n-icon size="36" color="#86909C"><CloudUploadOutline /></n-icon>
          </template>
          <template #extra>
            <n-text depth="3" style="font-size: 0.85rem">点击上方「备份数据库」创建备份</n-text>
          </template>
        </n-empty>
      </n-space>
    </div>
    <n-modal v-model:show="showCrlModal" preset="card" title="证书吊销列表 (CRL)" style="width: 700px">
      <div v-if="crlData">
        <n-descriptions :column="2" bordered>
          <n-descriptions-item label="版本">{{ crlData.version }}</n-descriptions-item>
          <n-descriptions-item label="吊销总数">{{ crlData.total_revoked }}</n-descriptions-item>
          <n-descriptions-item label="生成时间" :span="2">{{ new Date(crlData.generated_at).toLocaleString() }}</n-descriptions-item>
        </n-descriptions>
        <n-data-table
          v-if="crlData.revoked_certificates"
          :columns="crlColumns"
          :data="crlData.revoked_certificates"
          :max-height="300"
          style="margin-top: 12px"
        />
      </div>
    </n-modal>

    <!-- 证书吊销检查弹窗 -->
    <n-modal v-model:show="showCheckModal" preset="card" title="检查证书吊销状态" style="width: 500px">
      <n-space vertical>
        <n-input v-model:value="serialInput" placeholder="输入证书序列号" />
        <n-button type="primary" block @click="doCheckRevocation" :loading="checking">
          检查
        </n-button>
      </n-space>
      <n-alert v-if="revocationResult" :type="revocationResult.revoked ? 'error' : 'success'" style="margin-top: 12px">
        {{ revocationResult.revoked ? '✗ 该证书已被吊销' : '✓ 该证书有效，未被吊销' }}
      </n-alert>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, h, onMounted } from 'vue'
import { NIcon, NTag, NButton, NSpace, NEmpty, NDataTable, NText, NAlert, useMessage } from 'naive-ui'
import api, { certApi, caApi } from '@/api'
import {
  GlobeOutline, LockClosedOutline, ServerOutline, InformationCircleOutline,
  RefreshOutline, DownloadOutline, CloudUploadOutline, ShieldCheckmarkOutline,
  ListOutline, SearchOutline, CloudDownloadOutline,
} from '@vicons/ionicons5'

const message = useMessage()
const loadingCrl = ref(false)
const showCrlModal = ref(false)
const crlData = ref<any>(null)
const showCheckModal = ref(false)
const serialInput = ref('')
const checking = ref(false)
const revocationResult = ref<any>(null)

// === 备份管理 ===
const backing = ref(false)
const loadingBackups = ref(false)
const backups = ref<any[]>([])

const backupColumns = [
  { title: '文件名', key: 'filename', ellipsis: { tooltip: true } },
  {
    title: '大小',
    key: 'size',
    width: 120,
    render: (row: any) => formatSize(row.size),
  },
  {
    title: '备份时间',
    key: 'modified',
    width: 180,
    render: (row: any) => row.modified ? new Date(row.modified).toLocaleString('zh-CN') : '-',
  },
  {
    title: '操作',
    key: 'actions',
    width: 100,
    render: (row: any) =>
      h(NButton, {
        size: 'small',
        type: 'primary',
        quaternary: true,
        onClick: () => handleDownloadBackup(row.filename),
      }, { default: () => '下载' }),
  },
]

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

async function loadBackups() {
  loadingBackups.value = true
  try {
    const res = await caApi.listBackups()
    if (res.success && res.data) {
      backups.value = res.data || []
    }
  } catch (e: any) {
    message.error('加载备份列表失败: ' + e.message)
  } finally {
    loadingBackups.value = false
  }
}

function handleDownloadBackup(filename: string) {
  const link = document.createElement('a')
  link.href = caApi.downloadBackupUrl(filename)
  link.download = filename
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
}

const crlColumns = [
  { title: '序列号', key: 'serial_number', width: 180 },
  { title: 'SPIFFE ID', key: 'spiffe_id', ellipsis: { tooltip: true } },
  { title: '吊销时间', key: 'revoked_at', width: 180, render: (row: any) => row.revoked_at ? new Date(row.revoked_at).toLocaleString() : '无' },
  { title: '到期时间', key: 'expires_at', width: 180, render: (row: any) => row.expires_at ? new Date(row.expires_at).toLocaleString() : '无' },
]

function handleRegenCa() {
  const confirmed = window.confirm('重新生成 Root CA 将使所有设备证书失效，确定继续吗？')
  if (!confirmed) return
  message.loading('正在重新生成 CA...')
  api.post('/ca/regenerate').then(() => {
    message.success('Root CA 已重新生成，请重启服务以应用更改')
  }).catch((e: any) => {
    message.error('重新生成 CA 失败: ' + e.message)
  })
}
function handleDownloadCa() {
  const link = document.createElement('a')
  link.href = '/api/v1/ca/cert'
  link.download = 'ca.crt'
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
}
function handleBackup() {
  backing.value = true
  message.loading('正在备份数据库...')
  api.post('/ca/backup').then((res: any) => {
    if (res.success && res.data) {
      message.success('数据库备份成功')
      loadBackups()
    }
  }).catch((e: any) => {
    message.error('数据库备份失败: ' + e.message)
  }).finally(() => {
    backing.value = false
  })
}

async function handleViewCrl() {
  loadingCrl.value = true
  try {
    const res = await certApi.crl()
    if (res.success && res.data) {
      crlData.value = res.data
      showCrlModal.value = true
    }
  } catch (e: any) {
    message.error(e.message)
  } finally {
    loadingCrl.value = false
  }
}

function handleCheckRevocation() {
  revocationResult.value = null
  serialInput.value = ''
  showCheckModal.value = true
}

async function doCheckRevocation() {
  if (!serialInput.value) {
    message.warning('请输入证书序列号')
    return
  }
  checking.value = true
  try {
    const res = await certApi.checkRevocation(serialInput.value)
    if (res.success && res.data) {
      revocationResult.value = res.data
    }
  } catch (e: any) {
    message.error(e.message)
  } finally {
    checking.value = false
  }
}

onMounted(() => {
  loadBackups()
})
</script>

<style scoped>
.card-header {
  display: flex;
  align-items: center;
  gap: 8px;
}
.card-title {
  font-size: 1rem;
  font-weight: 600;
  color: #1D2129;
}
</style>
