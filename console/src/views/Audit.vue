<template>
  <div>
    <h2 class="page-title">安全审计</h2>

    <!-- 顶部操作栏 -->
    <n-space style="margin-bottom: 16px">
      <n-button type="primary" @click="loadData" :loading="loading">
        <template #icon><n-icon><RefreshOutline /></n-icon></template>
        刷新日志
      </n-button>
      <n-button @click="handleComplianceReport" :loading="loadingReport">
        <template #icon><n-icon><DocumentTextOutline /></n-icon></template>
        合规报表
      </n-button>
      <n-button @click="handleVerifyIntegrity" :loading="verifying">
        <template #icon><n-icon><ShieldCheckmarkOutline /></n-icon></template>
        验证完整性
      </n-button>
    </n-space>

    <!-- 审计链状态 -->
    <div v-if="integrityResult" class="card" style="margin-bottom: 16px">
      <div class="card-header" style="margin-bottom: 12px">
        <n-icon size="18" :color="integrityResult.audit_chain_valid ? '#18a058' : '#d03050'">
          <ShieldCheckmarkOutline />
        </n-icon>
        <span class="card-title">审计链完整性</span>
      </div>
      <n-alert :type="integrityResult.audit_chain_valid ? 'success' : 'error'" :show-icon="true">
        {{ integrityResult.audit_chain_valid ? '✓ 审计日志哈希链验证通过，数据完整无损' : '✗ 审计日志哈希链验证失败，数据可能已被篡改！' }}
        <template #header>
          验证时间: {{ new Date(integrityResult.checked_at).toLocaleString() }}
        </template>
      </n-alert>
    </div>

    <!-- 审计日志表格 -->
    <div class="card">
      <n-data-table :columns="columns" :data="logs" :loading="loading" :pagination="pagination">
        <template #empty>
          <n-empty description="暂无审计日志">
            <template #icon>
              <n-icon size="48" color="#86909C"><DocumentTextOutline /></n-icon>
            </template>
            <template #extra>
              <n-text depth="3" style="font-size: 0.85rem">设备注册、策略发布等操作将记录在此处</n-text>
            </template>
          </n-empty>
        </template>
      </n-data-table>
    </div>

    <!-- 合规报表弹窗 -->
    <n-modal v-model:show="showReportModal" preset="card" title="等保 2.0 三级合规报表" style="width: 700px">
      <div v-if="complianceReport">
        <n-descriptions :column="1" bordered>
          <n-descriptions-item label="报表生成时间">
            {{ new Date(complianceReport.generated_at || '').toLocaleString() }}
          </n-descriptions-item>
          <n-descriptions-item label="报表ID">
            {{ complianceReport.report_id || '-' }}
          </n-descriptions-item>
          <n-descriptions-item label="标准">
            {{ complianceReport.standard || 'GB/T 22239-2019' }}
          </n-descriptions-item>
          <n-descriptions-item label="总审计记录数">
            {{ complianceReport.summary?.total_audit_logs || 0 }}
          </n-descriptions-item>
          <n-descriptions-item label="总设备数">
            {{ complianceReport.summary?.total_devices || 0 }}
          </n-descriptions-item>
          <n-descriptions-item label="在线设备数">
            {{ complianceReport.summary?.online_devices || 0 }}
          </n-descriptions-item>
          <n-descriptions-item label="吊销设备数">
            {{ complianceReport.summary?.revoked_devices || 0 }}
          </n-descriptions-item>
          <n-descriptions-item label="活跃证书数">
            {{ complianceReport.summary?.active_certificates || 0 }}
          </n-descriptions-item>
          <n-descriptions-item label="已发布策略数">
            {{ complianceReport.summary?.published_policies || 0 }}
          </n-descriptions-item>
          <n-descriptions-item label="告警总数">
            {{ complianceReport.summary?.total_alerts || 0 }}
          </n-descriptions-item>
          <n-descriptions-item label="审计链完整性">
            <n-tag :type="complianceReport.summary?.audit_chain_valid ? 'success' : 'error'" size="small">
              {{ complianceReport.summary?.audit_chain_valid ? '✓ 完整' : '✗ 异常' }}
            </n-tag>
          </n-descriptions-item>
        </n-descriptions>

        <div v-if="complianceReport.compliance_items" style="margin-top: 16px">
          <div class="card-subtitle" style="margin-bottom: 12px">
            <n-icon size="16" color="#165DFF"><CheckmarkCircleOutline /></n-icon>
            <span>等保 2.0 控制项对照</span>
          </div>
          <n-data-table :columns="controlColumns" :data="complianceReport.compliance_items" :bordered="false" />
        </div>

        <n-space style="margin-top: 16px; justify-content: flex-end">
          <n-button @click="exportReport">
            <template #icon><n-icon><DownloadOutline /></n-icon></template>
            导出 JSON
          </n-button>
        </n-space>
      </div>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NIcon, useMessage } from 'naive-ui'
import {
  DocumentTextOutline, RefreshOutline, ShieldCheckmarkOutline,
  DownloadOutline, CheckmarkCircleOutline,
} from '@vicons/ionicons5'
import { auditApi, type AuditLog } from '@/api'

const message = useMessage()
const loading = ref(false)
const loadingReport = ref(false)
const verifying = ref(false)
const logs = ref<AuditLog[]>([])
const pagination = ref({ page: 1, pageSize: 50 })
const showReportModal = ref(false)
const complianceReport = ref<any>(null)
const integrityResult = ref<any>(null)

const columns = [
  {
    title: '时间',
    key: 'timestamp',
    width: 180,
    render: (row: AuditLog) => new Date(row.timestamp).toLocaleString(),
  },
  { title: '操作者', key: 'actor', width: 120 },
  { title: '操作', key: 'action', width: 150 },
  { title: '对象', key: 'target', ellipsis: { tooltip: true } },
  {
    title: '结果',
    key: 'result',
    width: 120,
    render: (row: AuditLog) => {
      const type = row.result === 'success' ? 'success' : row.result.includes('deny') ? 'error' : 'warning'
      const label = row.result === 'success' ? '成功' : row.result.includes('deny') ? '拒绝' : row.result === 'error' ? '失败' : row.result === 'pending' ? '待处理' : row.result
      return h(NTag, { type, size: 'small' }, { default: () => label })
    },
  },
  { title: 'IP', key: 'ip_address', width: 150 },
]

const controlColumns = [
  { title: '等保要求', key: 'requirement', width: 150 },
  { title: '对应能力', key: 'capability', width: 200 },
  { title: '状态', key: 'status', width: 80, render: (row: any) => h(NTag, { type: row.status === 'compliant' ? 'success' : 'warning', size: 'small' }, { default: () => row.status === 'compliant' ? '满足' : '不满足' }) },
  { title: '证据', key: 'evidence' },
]

async function loadData() {
  loading.value = true
  try {
    const res = await auditApi.list(1, 200)
    if (res.success && res.data) {
      logs.value = res.data.items || []
    }
  } catch (e: any) {
    message.error(e.message)
  } finally {
    loading.value = false
  }
}

async function handleComplianceReport() {
  loadingReport.value = true
  try {
    const res = await auditApi.complianceReport()
    if (res.success && res.data) {
      complianceReport.value = res.data
      showReportModal.value = true
    }
  } catch (e: any) {
    message.error(e.message)
  } finally {
    loadingReport.value = false
  }
}

async function handleVerifyIntegrity() {
  verifying.value = true
  try {
    const res = await auditApi.verifyIntegrity()
    if (res.success && res.data) {
      integrityResult.value = res.data
      if (res.data.audit_chain_valid) {
        message.success('审计链完整性验证通过')
      } else {
        message.error('审计链验证失败，数据可能已被篡改！')
      }
    }
  } catch (e: any) {
    message.error(e.message)
  } finally {
    verifying.value = false
  }
}

function exportReport() {
  if (!complianceReport.value) return
  const blob = new Blob([JSON.stringify(complianceReport.value, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `compliance-report-${new Date().toISOString().slice(0, 10)}.json`
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  URL.revokeObjectURL(url)
  message.success('合规报表已导出')
}

import { h } from 'vue'
import { NTag } from 'naive-ui'

onMounted(loadData)
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
.card-subtitle {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.9rem;
  font-weight: 600;
  color: #1D2129;
}
</style>
