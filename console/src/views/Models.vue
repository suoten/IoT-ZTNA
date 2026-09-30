<template>
  <div>
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px">
      <h2 class="page-title">模型管理</h2>
      <n-space>
        <n-button @click="handleFineTune" :loading="fineTuning" type="warning">
          <template #icon><n-icon><SparklesOutline /></n-icon></template>
          在线微调
        </n-button>
        <n-button type="primary" @click="showTrain = true">
          <template #icon><n-icon><SchoolOutline /></n-icon></template>
          训练模型
        </n-button>
      </n-space>
    </div>

    <!-- 引导提示 -->
    <n-alert type="info" style="margin-bottom: 16px" :bordered="false">
      <template #header>
        <span style="font-size: 0.85rem">💡 模型说明</span>
      </template>
      <div style="font-size: 0.8rem; color: #4E5969; line-height: 1.8">
        系统使用 Autoencoder 自编码器检测设备行为异常。训练后模型自动激活，行为评分 > 0.8 时触发安全策略。
        <br />
        可导出模型给 EdgeAgent Hub 等第三方系统使用，实现分布式 AI 行为检测。
      </div>
    </n-alert>

    <!-- 当前模型信息 -->
    <div class="card" style="margin-bottom: 16px">
      <n-card title="当前活跃模型" size="small">
        <n-descriptions :column="4" label-placement="left" v-if="modelInfo">
          <n-descriptions-item label="版本">{{ modelInfo.version }}</n-descriptions-item>
          <n-descriptions-item label="输入维度">{{ modelInfo.input_dim }}</n-descriptions-item>
          <n-descriptions-item label="潜在空间维度">{{ modelInfo.latent_dim }}</n-descriptions-item>
          <n-descriptions-item label="训练轮次">{{ modelInfo.epochs }}</n-descriptions-item>
          <n-descriptions-item label="下锚(误差中位数)">{{ modelInfo.calibration?.lower_anchor?.toFixed(6) }}</n-descriptions-item>
          <n-descriptions-item label="上锚(99.5百分位)">{{ modelInfo.calibration?.upper_anchor?.toFixed(6) }}</n-descriptions-item>
          <n-descriptions-item label="校准样本数">{{ modelInfo.calibration?.calibration_samples }}</n-descriptions-item>
          <n-descriptions-item label="创建时间">{{ modelInfo.created_at ? new Date(modelInfo.created_at).toLocaleString() : '-' }}</n-descriptions-item>
        </n-descriptions>
        <n-empty v-else description="暂无模型信息" />
      </n-card>
    </div>

    <!-- 模型版本列表 -->
    <div class="card">
      <n-data-table :columns="columns" :data="versions" :loading="loading">
        <template #empty>
          <n-empty description="还没有模型版本">
            <template #icon>
              <n-icon size="48" color="#86909C"><CubeOutline /></n-icon>
            </template>
            <template #extra>
              <n-button type="primary" size="small" @click="showTrain = true">
                <template #icon><n-icon><SchoolOutline /></n-icon></template>
                开始首次训练
              </n-button>
            </template>
          </n-empty>
        </template>
      </n-data-table>
    </div>

    <!-- 训练对话框 -->
    <n-modal v-model:show="showTrain" preset="dialog" title="训练 Autoencoder 模型" style="width: 600px">
      <n-form :model="trainForm" label-placement="left" label-width="120">
        <n-form-item label="训练轮次">
          <n-input-number v-model:value="trainForm.epochs" :min="10" :max="1000" placeholder="100" />
        </n-form-item>
        <n-form-item label="学习率">
          <n-input-number v-model:value="trainForm.learning_rate" :min="0.00001" :max="1" :step="0.0001" placeholder="0.001" />
        </n-form-item>
        <n-form-item label="批次大小">
          <n-input-number v-model:value="trainForm.batch_size" :min="1" :max="256" placeholder="32" />
        </n-form-item>
        <n-form-item label="创建者">
          <n-input v-model:value="trainForm.created_by" placeholder="如: admin" />
        </n-form-item>
        <n-form-item label="描述">
          <n-input v-model:value="trainForm.description" type="textarea" :rows="2" placeholder="模型描述" />
        </n-form-item>
      </n-form>
      <template #action>
        <n-button @click="showTrain = false">取消</n-button>
        <n-button type="primary" :loading="training" @click="handleTrain">开始训练</n-button>
      </template>
    </n-modal>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, h } from 'vue'
import { NTag, NButton, NSpace, NIcon, NCard, NDescriptions, NDescriptionsItem, NAlert, useMessage } from 'naive-ui'
import {
  SchoolOutline, CubeOutline, SparklesOutline,
  ArrowUndoOutline, DownloadOutline, TrashOutline
} from '@vicons/ionicons5'
import { modelApi } from '@/api'

const message = useMessage()
const loading = ref(false)
const training = ref(false)
const fineTuning = ref(false)
const showTrain = ref(false)
const modelInfo = ref<any>(null)
const versions = ref<any[]>([])

const trainForm = ref({
  epochs: 100,
  learning_rate: 0.001,
  batch_size: 32,
  created_by: 'admin',
  description: '',
})

const columns = [
  { title: '版本', key: 'version', width: 100 },
  {
    title: '状态',
    key: 'status',
    width: 100,
    render: (row: any) => {
      const type = row.status === 'active' ? 'success' : row.status === 'ready' ? 'info' : row.status === 'archived' ? 'default' : 'warning'
      const label = row.status === 'active' ? '已激活' : row.status === 'ready' ? '就绪' : row.status === 'archived' ? '已归档' : row.status
      return h(NTag, { type, size: 'small' }, () => label)
    },
  },
  { title: '训练轮次', key: 'epochs', width: 100 },
  { title: '训练样本数', key: 'training_samples', width: 120 },
  { title: '验证损失', key: 'validation_loss', width: 120, render: (row: any) => row.validation_loss?.toFixed(6) },
  { title: '创建者', key: 'created_by', width: 100 },
  { title: '创建时间', key: 'created_at', width: 180, render: (row: any) => new Date(row.created_at).toLocaleString() },
  {
    title: '操作',
    key: 'actions',
    width: 200,
    render: (row: any) =>
      h(NSpace, {}, () => [
        row.status !== 'active' ? h(NButton, {
          size: 'small', type: 'success', onClick: () => handleActivate(row),
        }, () => '激活') : null,
        row.status !== 'active' ? h(NButton, {
          size: 'small', type: 'warning', onClick: () => handleRollback(row),
        }, () => '回滚') : null,
        h(NButton, {
          size: 'small', type: 'info', onClick: () => handleExport(row),
        }, () => '导出'),
        row.status !== 'active' ? h(NButton, {
          size: 'small', type: 'error', onClick: () => handleDelete(row),
        }, () => '删除') : null,
      ]),
  },
]

async function loadData() {
  loading.value = true
  try {
    const [infoRes, versionsRes] = await Promise.all([
      modelApi.getInfo(),
      modelApi.listVersions(),
    ])
    if (infoRes.success && infoRes.data) modelInfo.value = infoRes.data
    if (versionsRes.success && versionsRes.data) versions.value = versionsRes.data || []
  } catch (e: any) {
    message.error(e.message)
  } finally {
    loading.value = false
  }
}

async function handleTrain() {
  training.value = true
  try {
    const res = await modelApi.train(trainForm.value)
    if (res.success) {
      message.success(`训练完成! 损失: ${res.data.final_loss?.toFixed(6)}, 耗时: ${res.data.duration_secs?.toFixed(1)}s`)
      showTrain.value = false
      loadData()
    }
  } catch (e: any) {
    message.error(`训练失败: ${e.message}`)
  } finally {
    training.value = false
  }
}

async function handleFineTune() {
  fineTuning.value = true
  try {
    const res = await modelApi.fineTune({ created_by: 'admin' })
    if (res.success) {
      message.success(`微调完成! 损失: ${res.data.final_loss?.toFixed(6)}`)
      loadData()
    }
  } catch (e: any) {
    message.error(`微调失败: ${e.message}`)
  } finally {
    fineTuning.value = false
  }
}

async function handleActivate(row: any) {
  try {
    await modelApi.activate(row.id)
    message.success(`模型 ${row.version} 已激活`)
    loadData()
  } catch (e: any) {
    message.error(e.message)
  }
}

async function handleRollback(row: any) {
  try {
    await modelApi.rollback(row.id)
    message.success(`已回滚到 ${row.version}`)
    loadData()
  } catch (e: any) {
    message.error(e.message)
  }
}

async function handleExport(row: any) {
  try {
    const res = await modelApi.export(row.id)
    if (res.success) {
      const blob = new Blob([atob(res.data.data)], { type: 'application/octet-stream' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `model-${row.version}.onnx.json`
      a.click()
      URL.revokeObjectURL(url)
      message.success('模型已导出')
    }
  } catch (e: any) {
    message.error(e.message)
  }
}

async function handleDelete(row: any) {
  const confirmed = window.confirm(`确定要删除模型版本 ${row.version} 吗？`)
  if (!confirmed) return
  try {
    await modelApi.delete(row.id)
    message.success('模型版本已删除')
    loadData()
  } catch (e: any) {
    message.error(e.message)
  }
}

onMounted(loadData)
</script>
