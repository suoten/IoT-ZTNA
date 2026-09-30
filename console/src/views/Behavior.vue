<template>
  <div>
    <h2 class="page-title">行为检测</h2>

    <!-- 统计卡片 -->
    <n-grid :cols="3" :x-gap="16" :y-gap="16" style="margin-bottom: 16px">
      <n-grid-item>
        <div class="stat-card" style="--accent: #165DFF">
          <div class="stat-icon">
            <n-icon :size="24" color="#165DFF"><PulseOutline /></n-icon>
          </div>
          <div class="stat-body">
            <div class="stat-value" style="color: #165DFF">{{ stats.totalEvents }}</div>
            <div class="stat-label">总行为事件</div>
          </div>
        </div>
      </n-grid-item>
      <n-grid-item>
        <div class="stat-card" style="--accent: #18A058">
          <div class="stat-icon">
            <n-icon :size="24" color="#18A058"><HardwareChipOutline /></n-icon>
          </div>
          <div class="stat-body">
            <div class="stat-value" style="color: #18A058">{{ stats.devicesWithHistory }}</div>
            <div class="stat-label">有历史记录设备</div>
          </div>
        </div>
      </n-grid-item>
      <n-grid-item>
        <div class="stat-card" style="--accent: #F0A020">
          <div class="stat-icon">
            <n-icon :size="24" color="#F0A020"><AnalyticsOutline /></n-icon>
          </div>
          <div class="stat-body">
            <div class="stat-value" style="color: #F0A020">{{ stats.devicesWithBaseline }}</div>
            <div class="stat-label">已建立基线设备</div>
          </div>
        </div>
      </n-grid-item>
    </n-grid>

    <!-- 行为评估 -->
    <div class="card">
      <div class="card-header" style="margin-bottom: 16px">
        <n-icon size="18" color="#165DFF"><PulseOutline /></n-icon>
        <span class="card-title">设备行为评估</span>
      </div>
      <n-space align="center" style="margin-bottom: 16px">
        <n-input v-model:value="spiffeInput" placeholder="输入设备 SPIFFE ID" style="width: 400px">
          <template #prefix><n-icon color="#86909C"><SearchOutline /></n-icon></template>
        </n-input>
        <n-button type="primary" :loading="assessing" @click="handleAssess">
          <template #icon><n-icon><SearchOutline /></n-icon></template>
          评估
        </n-button>
        <n-button :loading="learningBaseline" @click="handleLearnBaseline">
          <template #icon><n-icon><SchoolOutline /></n-icon></template>
          学习基线
        </n-button>
        <n-button quaternary @click="handleGetBaseline">
          <template #icon><n-icon><EyeOutline /></n-icon></template>
          查看基线
        </n-button>
      </n-space>

      <!-- 评估结果 -->
      <div v-if="assessment" style="margin-top: 16px">
        <n-grid :cols="2" :x-gap="24">
          <n-grid-item>
            <n-descriptions :column="1" bordered>
              <n-descriptions-item label="异常评分">
                <div style="display: flex; align-items: center; gap: 12px">
                  <n-progress
                    type="circle"
                    :percentage="Math.round(assessment.anomaly_score * 100)"
                    :color="assessment.anomaly_score > 0.7 ? '#d03050' : assessment.anomaly_score > 0.3 ? '#f0a020' : '#18a058'"
                    :stroke-width="6"
                    :show-indicator="true"
                    style="width: 48px; height: 48px"
                  />
                  <span :style="{ color: assessment.anomaly_score > 0.7 ? '#d03050' : assessment.anomaly_score > 0.3 ? '#f0a020' : '#18a058', fontWeight: 700, fontSize: '1.2rem' }">
                    {{ assessment.anomaly_score.toFixed(2) }}
                  </span>
                </div>
              </n-descriptions-item>
              <n-descriptions-item label="是否异常">
                <n-tag :type="assessment.is_anomaly ? 'error' : 'success'" size="small">
                  {{ assessment.is_anomaly ? '异常' : '正常' }}
                </n-tag>
              </n-descriptions-item>
              <n-descriptions-item label="建议操作">
                <n-tag :type="decisionTagType(assessment.recommended_action)" size="small">
                  {{ decisionLabel(assessment.recommended_action) }}
                </n-tag>
              </n-descriptions-item>
              <n-descriptions-item label="异常类型">
                <n-space v-if="assessment.anomaly_types.length">
                  <n-tag v-for="t in assessment.anomaly_types" :key="t" type="warning" size="small">{{ t }}</n-tag>
                </n-space>
                <span v-else>无</span>
              </n-descriptions-item>
              <n-descriptions-item label="原因">{{ assessment.reason }}</n-descriptions-item>
            </n-descriptions>
          </n-grid-item>
          <n-grid-item>
            <!-- SHAP 风格特征贡献图 -->
            <div v-if="assessment.feature_contributions && assessment.feature_contributions.length">
              <div class="card-subtitle">
                <n-icon size="16" color="#165DFF"><BarChartOutline /></n-icon>
                <span>特征贡献分析（SHAP 风格）</span>
              </div>
              <div ref="shapChartRef" style="width: 100%; height: 280px; margin-top: 12px"></div>
            </div>
            <n-empty v-else description="无特征贡献数据" style="margin-top: 24px">
              <template #icon>
                <n-icon size="36" color="#86909C"><BarChartOutline /></n-icon>
              </template>
            </n-empty>
          </n-grid-item>
        </n-grid>
      </div>
      <n-empty v-else description="暂无评估结果" style="margin-top: 24px">
        <template #icon>
          <n-icon size="36" color="#86909C"><AnalyticsOutline /></n-icon>
        </template>
        <template #extra>
          <n-text depth="3" style="font-size: 0.85rem">输入设备的 SPIFFE ID 并点击"评估"按钮，查看设备行为分析结果</n-text>
        </template>
      </n-empty>
    </div>

    <!-- 基线信息弹窗 -->
    <n-modal v-model:show="showBaselineModal" preset="card" title="设备行为基线" style="width: 600px">
      <n-descriptions v-if="baselineData" :column="1" bordered>
        <n-descriptions-item label="SPIFFE ID">{{ baselineData.spiffe_id }}</n-descriptions-item>
        <n-descriptions-item label="协议频率">{{ baselineData.baseline?.protocol_rates }}</n-descriptions-item>
        <n-descriptions-item label="常用操作">{{ baselineData.baseline?.common_operations?.join(', ') }}</n-descriptions-item>
        <n-descriptions-item label="常用目标">{{ baselineData.baseline?.common_targets?.join(', ') }}</n-descriptions-item>
        <n-descriptions-item label="平均数据大小">{{ (baselineData.baseline?.avg_data_size || 0).toFixed(0) }} 字节</n-descriptions-item>
        <n-descriptions-item label="活跃时段">{{ baselineData.baseline?.active_hours ? `${baselineData.baseline.active_hours[0]}:00 - ${baselineData.baseline.active_hours[1]}:00` : '无' }}</n-descriptions-item>
        <n-descriptions-item label="异常阈值">{{ baselineData.baseline?.anomaly_threshold }}</n-descriptions-item>
        <n-descriptions-item label="更新时间">{{ baselineData.baseline?.updated_at }}</n-descriptions-item>
      </n-descriptions>
      <n-empty v-else description="尚无基线数据，请先点击'学习基线'" />
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick, watch } from 'vue'
import { NIcon, useMessage } from 'naive-ui'
import {
  PulseOutline, HardwareChipOutline, AnalyticsOutline, SearchOutline,
  SchoolOutline, EyeOutline, BarChartOutline,
} from '@vicons/ionicons5'
import { behaviorApi } from '@/api'
import * as echarts from 'echarts'

const message = useMessage()
const stats = ref({ totalEvents: 0, devicesWithHistory: 0, devicesWithBaseline: 0 })
const spiffeInput = ref('')
const assessment = ref<any>(null)
const assessing = ref(false)
const learningBaseline = ref(false)
const showBaselineModal = ref(false)
const baselineData = ref<any>(null)
const shapChartRef = ref<HTMLElement | null>(null)
let shapChart: echarts.ECharts | null = null

async function loadStats() {
  try {
    const res = await behaviorApi.stats()
    if (res.success && res.data) {
      stats.value = {
        totalEvents: res.data.total_events || 0,
        devicesWithHistory: res.data.devices_with_history || 0,
        devicesWithBaseline: res.data.devices_with_baseline || 0,
      }
    }
  } catch (e: any) {
    message.error(e.message)
  }
}

async function handleAssess() {
  if (!spiffeInput.value) {
    message.warning('请输入 SPIFFE ID')
    return
  }
  assessing.value = true
  try {
    const res = await behaviorApi.assess(spiffeInput.value)
    if (res.success && res.data) {
      assessment.value = res.data
      await nextTick()
      renderShapChart()
    }
  } catch (e: any) {
    message.error(e.message)
  } finally {
    assessing.value = false
  }
}

async function handleLearnBaseline() {
  if (!spiffeInput.value) {
    message.warning('请输入 SPIFFE ID')
    return
  }
  learningBaseline.value = true
  try {
    const res = await behaviorApi.updateBaseline(spiffeInput.value)
    if (res.success && res.data) {
      message.success('行为基线已更新')
      baselineData.value = res.data
      await loadStats()
    }
  } catch (e: any) {
    message.error(e.message)
  } finally {
    learningBaseline.value = false
  }
}

async function handleGetBaseline() {
  if (!spiffeInput.value) {
    message.warning('请输入 SPIFFE ID')
    return
  }
  try {
    const res = await behaviorApi.getBaseline(spiffeInput.value)
    if (res.success && res.data) {
      baselineData.value = res.data
      showBaselineModal.value = true
    }
  } catch (e: any) {
    message.error(e.message)
  }
}

function renderShapChart() {
  if (!shapChartRef.value || !assessment.value?.feature_contributions) return

  if (shapChart) {
    shapChart.dispose()
  }
  shapChart = echarts.init(shapChartRef.value)

  const contributions = assessment.value.feature_contributions
  const features = contributions.map((c: any) => c.feature)
  const values = contributions.map((c: any) => ({
    value: parseFloat(c.contribution || 0),
    itemStyle: {
      color: (c.contribution || 0) > 0.15 ? '#d03050' : (c.contribution || 0) > 0.05 ? '#f0a020' : '#18a058',
    },
  }))

  const option = {
    tooltip: {
      trigger: 'axis',
      formatter: (params: any) => {
        const idx = params[0].dataIndex
        const c = contributions[idx]
        return `<b>${c.feature}</b><br/>贡献度: ${(c.contribution || 0).toFixed(3)}<br/>当前值: ${c.value}<br/>基线值: ${c.baseline_value}`
      },
    },
    grid: { left: '15%', right: '10%', top: '10%', bottom: '10%' },
    xAxis: { type: 'value', max: 0.5, name: '贡献度' },
    yAxis: { type: 'category', data: features, axisLabel: { fontSize: 12 } },
    series: [{
      type: 'bar',
      data: values.map((v: any) => v.value),
      itemStyle: {
        color: (params: any) => {
          const c = contributions[params.dataIndex]
          return (c.contribution || 0) > 0.15 ? '#d03050' : (c.contribution || 0) > 0.05 ? '#f0a020' : '#18a058'
        },
      },
      label: { show: true, position: 'right', formatter: '{c}' },
    }],
  }

  shapChart.setOption(option)
}

// 窗口 resize 时重绘图表
window.addEventListener('resize', () => {
  if (shapChart) shapChart.resize()
})

watch(() => assessment.value, () => {
  nextTick(() => renderShapChart())
})

function decisionTagType(decision: string) {
  switch (decision) {
    case 'allow': return 'success'
    case 'deny': return 'error'
    case 'log': return 'warning'
    case 'redirect': return 'info'
    default: return 'default'
  }
}

function decisionLabel(decision: string): string {
  const map: Record<string, string> = {
    'allow': '允许',
    'deny': '拒绝',
    'log': '记录',
    'redirect': '重定向',
  }
  return map[decision?.toLowerCase()] || decision || '-'
}

onMounted(loadStats)
</script>

<style scoped>
.stat-card {
  display: flex;
  align-items: center;
  gap: 16px;
  background: #fff;
  border-radius: 12px;
  padding: 20px 24px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.06);
}
.stat-card:hover {
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.1);
}
.stat-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, var(--accent) 12%, transparent);
  flex-shrink: 0;
}
.stat-value {
  font-size: 1.8rem;
  font-weight: 700;
  line-height: 1.2;
}
.stat-label {
  font-size: 0.85rem;
  color: #86909C;
  margin-top: 2px;
}
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
