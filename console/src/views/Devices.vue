<template>
  <div>
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px">
      <h2 class="page-title">设备管理</h2>
      <n-button type="primary" @click="showRegister = true">
        <template #icon><n-icon><AddOutline /></n-icon></template>
        注册设备
      </n-button>
    </div>

    <!-- 引导提示 -->
    <n-alert type="info" style="margin-bottom: 16px" :bordered="false">
      <template #header>
        <span style="font-size: 0.85rem">💡 新手指南</span>
      </template>
      <div style="font-size: 0.8rem; color: #4E5969; line-height: 1.8">
        注册设备后系统会自动签发 SPIFFE 证书，设备凭证书接入网关。
        <br />
        设备状态：在线 / 离线 / 已吊销 / 待激活 — 点击「注册设备」添加新设备。
      </div>
    </n-alert>

    <div class="card">
      <n-data-table
        :columns="columns"
        :data="devices"
        :loading="loading"
        :pagination="pagination"
        @update:page="handlePageChange"
      >
        <template #empty>
          <n-empty description="还没有设备">
            <template #icon>
              <n-icon size="48" color="#86909C"><HardwareChipOutline /></n-icon>
            </template>
            <template #extra>
              <n-button type="primary" size="small" @click="showRegister = true">
                <template #icon><n-icon><AddOutline /></n-icon></template>
                注册第一台设备
              </n-button>
            </template>
          </n-empty>
        </template>
      </n-data-table>
    </div>

    <!-- 注册设备对话框 -->
    <n-modal v-model:show="showRegister" preset="dialog" title="注册新设备" style="width: 500px">
      <n-form ref="formRef" :model="formData" label-placement="left" label-width="100">
        <n-form-item label="序列号" path="serial_number">
          <n-input v-model:value="formData.serial_number" placeholder="如: SN-A001-2026" />
        </n-form-item>
        <n-form-item label="型号" path="model">
          <n-input v-model:value="formData.model" placeholder="如: RTU-Gateway-V2" />
        </n-form-item>
        <n-form-item label="信任根" path="trust_root">
          <n-select v-model:value="formData.trust_root" :options="trustRootOptions" />
        </n-form-item>
        <n-form-item label="信任等级" path="trust_level">
          <n-select v-model:value="formData.trust_level" :options="trustLevelOptions" />
        </n-form-item>
        <n-form-item label="能力" path="capabilities">
          <n-dynamic-tags v-model:value="formData.capabilities" />
        </n-form-item>
        <n-form-item label="硬件指纹" path="hardware_fingerprint">
          <n-input v-model:value="formData.hardware_fingerprint" placeholder="SHA256" />
        </n-form-item>
      </n-form>
      <template #action>
        <n-button @click="showRegister = false">取消</n-button>
        <n-button type="primary" :loading="registering" @click="handleRegister">注册</n-button>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, h } from 'vue'
import { NTag, NButton, NSpace, NIcon, NAlert, useMessage } from 'naive-ui'
import { AddOutline, SettingsOutline, BanOutline, ChevronForwardOutline, HardwareChipOutline } from '@vicons/ionicons5'
import api, { deviceApi, type Device } from '@/api'

const message = useMessage()
const loading = ref(false)
const devices = ref<Device[]>([])
const showRegister = ref(false)
const registering = ref(false)
const pagination = ref({ page: 1, pageSize: 20, itemCount: 0, showSizePicker: false })

const trustRootOptions = [
  { label: 'TPM 2.0', value: 'TPM' },
  { label: '安全元件 (SE)', value: 'SE' },
  { label: 'Flash 烧录', value: 'FLASH' },
]
const trustLevelOptions = [
  { label: 'A级 - ARM Linux', value: 'A' },
  { label: 'B级 - Cortex-M', value: 'B' },
  { label: 'C级 - ESP32/受限', value: 'C' },
]

const formData = ref({
  serial_number: '',
  model: '',
  trust_root: 'TPM',
  trust_level: 'A',
  capabilities: [] as string[],
  hardware_fingerprint: '',
})

const columns = [
  { title: '序列号', key: 'serial_number', width: 180 },
  { title: 'SPIFFE ID', key: 'spiffe_id', ellipsis: { tooltip: true } },
  { title: '型号', key: 'model', width: 150 },
  {
    title: '等级',
    key: 'trust_level',
    width: 80,
    render: (row: Device) => h('span', { class: `level-badge level-${row.trust_level}` }, row.trust_level),
  },
  {
    title: '状态',
    key: 'status',
    width: 100,
    render: (row: Device) => {
      const label = row.status === 'online' ? '在线' : row.status === 'offline' ? '离线' : row.status === 'revoked' ? '已吊销' : row.status === 'pending' ? '待激活' : row.status
      return h('span', { class: `status-badge status-${row.status}` }, label)
    },
  },
  {
    title: '注册时间',
    key: 'registered_at',
    width: 180,
    render: (row: Device) => new Date(row.registered_at).toLocaleString(),
  },
  {
    title: '操作',
    key: 'actions',
    width: 200,
    render: (row: Device) =>
      h(NSpace, {}, () => [
        h(NButton, {
          size: 'small',
          type: row.maintenance_mode ? 'warning' : 'default',
          onClick: () => handleMaintenance(row),
        }, () => row.maintenance_mode ? '退出维护' : '维护模式'),
        h(NButton, {
          size: 'small',
          type: 'error',
          disabled: row.status === 'revoked',
          onClick: () => handleRevoke(row),
        }, () => '吊销'),
      ]),
  },
]

async function loadData() {
  loading.value = true
  try {
    const res = await deviceApi.list(pagination.value.page, pagination.value.pageSize)
    if (res.success && res.data) {
      devices.value = res.data.items || []
      pagination.value.itemCount = res.data.total || 0
    }
  } catch (e: any) {
    message.error(e.message)
  } finally {
    loading.value = false
  }
}

function handlePageChange(page: number) {
  pagination.value.page = page
  loadData()
}

async function handleRegister() {
  registering.value = true
  try {
    const res = await deviceApi.register(formData.value)
    if (res.success) {
      message.success('设备注册成功')
      showRegister.value = false
      loadData()
      formData.value = {
        serial_number: '', model: '', trust_root: 'TPM',
        trust_level: 'A', capabilities: [], hardware_fingerprint: '',
      }
    }
  } catch (e: any) {
    message.error(`注册失败: ${e.message}`)
  } finally {
    registering.value = false
  }
}

async function handleMaintenance(row: Device) {
  try {
    await deviceApi.setMaintenance(row.serial_number, !row.maintenance_mode)
    message.success(row.maintenance_mode ? '已退出维护模式' : '已进入维护模式')
    loadData()
  } catch (e: any) {
    message.error(e.message)
  }
}

async function handleRevoke(row: Device) {
  const confirmed = window.confirm(`确定要吊销设备 ${row.serial_number} 吗？此操作不可逆。`)
  if (!confirmed) return
  try {
    await deviceApi.revoke(row.serial_number)
    message.success('设备已吊销')
    loadData()
  } catch (e: any) {
    message.error(e.message)
  }
}

onMounted(loadData)
</script>
