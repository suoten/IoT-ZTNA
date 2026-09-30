<template>
  <div>
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px">
      <h2 class="page-title">Rego 策略引擎</h2>
      <n-space>
        <n-button @click="loadRevision" :loading="revLoading">
          <template #icon><n-icon><RefreshOutline /></n-icon></template>
          刷新版本
        </n-button>
        <n-button type="warning" @click="showBundle = true">
          <template #icon><n-icon><CubeOutline /></n-icon></template>
          Bundle 管理
        </n-button>
        <n-button type="primary" @click="activeTab = 'editor'">
          <template #icon><n-icon><CodeSlashOutline /></n-icon></template>
          策略编辑器
        </n-button>
      </n-space>
    </div>

    <!-- 引擎状态卡片 -->
    <div class="card" style="margin-bottom: 16px">
      <n-card title="Rego 引擎状态" size="small">
        <n-descriptions :column="3" label-placement="left">
          <n-descriptions-item label="引擎版本">
            <n-tag type="info" size="small">{{ revision?.revision || '加载中...' }}</n-tag>
          </n-descriptions-item>
          <n-descriptions-item label="当前 Bundle">
            <n-tag type="success" size="small">{{ revision?.current_bundle || '-' }}</n-tag>
          </n-descriptions-item>
          <n-descriptions-item label="规则数量">
            <n-tag type="warning" size="small">{{ rules.length }} 条</n-tag>
          </n-descriptions-item>
        </n-descriptions>
      </n-card>
    </div>

    <!-- Tab 区域 -->
    <div class="card">
      <n-tabs v-model:value="activeTab" type="line" animated>
        <!-- === 策略编辑器 === -->
        <n-tab-pane name="editor" tab="策略编辑器">
          <n-space vertical :size="16">
            <n-space>
              <n-button size="small" @click="loadDefaultRules">
                <template #icon><n-icon><DocumentTextOutline /></n-icon></template>
                加载默认模板
              </n-button>
              <n-button size="small" type="success" @click="addRule">
                <template #icon><n-icon><AddOutline /></n-icon></template>
                新增规则
              </n-button>
              <n-button size="small" type="primary" @click="showSimulateForEditor = true">
                <template #icon><n-icon><FlashOutline /></n-icon></template>
                模拟测试
              </n-button>
              <n-button size="small" type="warning" @click="handlePackBundle">
                <template #icon><n-icon><CubeOutline /></n-icon></template>
                打包为 Bundle
              </n-button>
            </n-space>

            <!-- 规则列表 -->
            <div v-for="(rule, index) in rules" :key="index" class="rule-item">
              <n-card size="small" :bordered="true">
                <template #header>
                  <n-input
                    v-model:value="rule.name"
                    size="small"
                    placeholder="规则名称"
                    style="width: 250px"
                  />
                </template>
                <template #header-extra>
                  <n-space align="center">
                    <n-tag :type="rule.decision === 'allow' ? 'success' : rule.decision === 'deny' ? 'error' : 'info'" size="small">
                      {{ rule.decision }}
                    </n-tag>
                    <n-input-number v-model:value="rule.priority" :min="0" :max="100" size="small" style="width: 100px" />
                    <n-button size="small" type="error" quaternary @click="removeRule(index)">
                      <template #icon><n-icon><TrashOutline /></n-icon></template>
                    </n-button>
                  </n-space>
                </template>

                <n-space vertical :size="8">
                  <n-input v-model:value="rule.description" size="small" placeholder="规则描述" />
                  <n-input
                    v-model:value="rule.rego"
                    type="textarea"
                    :rows="3"
                    placeholder='Rego 规则内容，如: allow if input.protocol == "modbus"'
                    style="font-family: 'Cascadia Code', 'Fira Code', monospace; font-size: 13px"
                  />
                  <n-select
                    v-model:value="rule.decision"
                    :options="decisionOptions"
                    size="small"
                    style="width: 150px"
                  />
                </n-space>
              </n-card>
            </div>

            <n-empty v-if="rules.length === 0" description="还没有规则，点击「加载默认模板」或「新增规则」开始">
              <template #icon>
                <n-icon size="48" color="#86909C"><CodeSlashOutline /></n-icon>
              </template>
            </n-empty>
          </n-space>
        </n-tab-pane>

        <!-- === 策略模拟器 === -->
        <n-tab-pane name="simulator" tab="策略模拟器">
          <n-grid :cols="2" :x-gap="16" :y-gap="16">
            <n-grid-item>
              <n-card title="输入上下文 (PolicyRequest)" size="small">
                <n-form :model="simulateInput" label-placement="left" label-width="120" size="small">
                  <n-form-item label="SPIFFE ID">
                    <n-input v-model:value="simulateInput.device_spiffe_id" placeholder="spiffe://iot-ztna/device/SN-A001" />
                  </n-form-item>
                  <n-form-item label="协议">
                    <n-select v-model:value="simulateInput.protocol" :options="protocolOptions" />
                  </n-form-item>
                  <n-form-item label="操作">
                    <n-select v-model:value="simulateInput.operation" :options="operationOptions" />
                  </n-form-item>
                  <n-form-item label="目标地址">
                    <n-input v-model:value="simulateInput.target" placeholder="192.168.1.100:502" />
                  </n-form-item>
                  <n-form-item label="功能码">
                    <n-input-number v-model:value="simulateInput.function_code" :min="0" :max="127" style="width: 100%" />
                  </n-form-item>
                  <n-form-item label="MQTT Topic">
                    <n-input v-model:value="simulateInput.topic" placeholder="sensors/temp/telemetry" />
                  </n-form-item>
                  <n-form-item label="行为评分">
                    <n-input-number v-model:value="simulateInput.behavior_score" :min="0" :max="1" :step="0.01" style="width: 100%" />
                  </n-form-item>
                  <n-form-item label="维护模式">
                    <n-switch v-model:value="simulateInput.maintenance_mode" />
                  </n-form-item>
                  <n-form-item label=" ">
                    <n-button type="primary" @click="handleSimulate" :loading="simulating" block>
                      <template #icon><n-icon><FlashOutline /></n-icon></template>
                      执行模拟
                    </n-button>
                  </n-form-item>
                </n-form>
              </n-card>
            </n-grid-item>

            <n-grid-item>
              <n-card title="模拟结果" size="small">
                <div v-if="simulateResult" class="simulate-result">
                  <n-space vertical :size="16">
                    <div>
                      <span class="result-label">决策结果：</span>
                      <n-tag
                        :type="simulateResult.decision === 'allow' ? 'success' : simulateResult.decision === 'deny' ? 'error' : 'warning'"
                        size="large"
                      >
                        {{ decisionLabel(simulateResult.decision) }}
                      </n-tag>
                    </div>
                    <div>
                      <span class="result-label">匹配原因：</span>
                      <span class="result-value">{{ simulateResult.reason }}</span>
                    </div>
                    <div>
                      <span class="result-label">匹配规则：</span>
                      <n-space>
                        <n-tag v-for="rule in simulateResult.matched_rules" :key="rule" type="info" size="small">
                          {{ rule }}
                        </n-tag>
                        <span v-if="!simulateResult.matched_rules?.length" style="color: #86909C">无匹配</span>
                      </n-space>
                    </div>
                    <div>
                      <span class="result-label">时间戳：</span>
                      <span class="result-value">{{ new Date(simulateResult.timestamp).toLocaleString() }}</span>
                    </div>
                  </n-space>
                </div>
                <n-empty v-else description="点击「执行模拟」查看策略决策结果">
                  <template #icon>
                    <n-icon size="48" color="#86909C"><FlashOutline /></n-icon>
                  </template>
                </n-empty>
              </n-card>
            </n-grid-item>
          </n-grid>
        </n-tab-pane>

        <!-- === Bundle 管理 === -->
        <n-tab-pane name="bundle" tab="Bundle 管理">
          <n-space vertical :size="16">
            <n-alert type="info" :bordered="false">
              Bundle 是策略打包格式。打包后的 Bundle 可以热加载到引擎中，无需重启服务。支持本地打包与热加载全流程。
            </n-alert>

            <n-space>
              <n-button type="primary" @click="showBundlePack = true">
                <template #icon><n-icon><CubeOutline /></n-icon></template>
                打包 Bundle
              </n-button>
              <n-button type="warning" @click="handleSyncBundle" :loading="syncing">
                <template #icon><n-icon><CloudDownloadOutline /></n-icon></template>
                重载默认策略
              </n-button>
              <n-button @click="loadRevision">
                <template #icon><n-icon><RefreshOutline /></n-icon></template>
                刷新状态
              </n-button>
            </n-space>

            <n-card title="热加载操作" size="small">
              <n-space align="center">
                <n-input
                  v-model:value="hotLoadRevision"
                  placeholder="输入要热加载的 Bundle revision"
                  style="width: 400px"
                />
                <n-button type="success" @click="handleHotLoad" :loading="hotLoading">
                  <template #icon><n-icon><SwapHorizontalOutline /></n-icon></template>
                  热加载
                </n-button>
              </n-space>
            </n-card>

            <!-- Bundle 历史 -->
            <n-card title="Bundle 历史" size="small" v-if="bundleHistory.length > 0">
              <n-data-table :columns="bundleColumns" :data="bundleHistory" size="small" />
            </n-card>
          </n-space>
        </n-tab-pane>

        <!-- === 语法帮助 === -->
        <n-tab-pane name="help" tab="语法帮助">
          <n-card size="small">
            <n-space vertical :size="12">
              <h3 style="margin: 0; color: #1D2129">支持的 Rego 语法</h3>

              <n-card title="基本比较" size="small" :bordered="false" class="help-card">
                <n-code language="rego" :code="helpCode1" />
              </n-card>

              <n-card title="逻辑组合 (and)" size="small" :bordered="false" class="help-card">
                <n-code language="rego" :code="helpCode2" />
              </n-card>

              <n-card title="正则匹配 (matches)" size="small" :bordered="false" class="help-card">
                <n-code language="rego" :code="helpCode3" />
              </n-card>

              <n-card title="列表包含 (in)" size="small" :bordered="false" class="help-card">
                <n-code language="rego" :code="helpCode4" />
              </n-card>

              <n-card title="无条件规则" size="small" :bordered="false" class="help-card">
                <n-code language="rego" :code="helpCode5" />
                <n-text depth="3" style="font-size: 12px">无条件 deny 规则可用作默认拒绝兜底</n-text>
              </n-card>

              <n-card title="可用 Input 字段" size="small" :bordered="false" class="help-card">
                <n-table :bordered="false" :single-line="false" size="small">
                  <thead>
                    <tr>
                      <th>字段</th>
                      <th>类型</th>
                      <th>说明</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr><td>input.spiffe_id</td><td>String</td><td>设备 SPIFFE ID</td></tr>
                    <tr><td>input.protocol</td><td>String</td><td>协议: modbus, mqtt, http 等</td></tr>
                    <tr><td>input.operation</td><td>String</td><td>操作: read, write, publish, subscribe</td></tr>
                    <tr><td>input.target</td><td>String</td><td>目标地址</td></tr>
                    <tr><td>input.function_code</td><td>Number</td><td>Modbus 功能码</td></tr>
                    <tr><td>input.topic</td><td>String</td><td>MQTT Topic</td></tr>
                    <tr><td>input.behavior_score</td><td>Number</td><td>AI 行为异常评分 (0~1)</td></tr>
                    <tr><td>input.maintenance_mode</td><td>Boolean</td><td>是否维护模式</td></tr>
                  </tbody>
                </n-table>
              </n-card>

              <n-alert type="warning" :bordered="false">
                优先级数值越大越高。第一个匹配的高优先级规则决定最终决策。
              </n-alert>
            </n-space>
          </n-card>
        </n-tab-pane>
      </n-tabs>
    </div>

    <!-- Bundle 打包对话框 -->
    <n-modal v-model:show="showBundlePack" preset="dialog" title="打包 Rego Bundle" style="width: 500px">
      <n-alert type="info" :bordered="false" style="margin-bottom: 16px">
        将当前编辑器中的 {{ rules.length }} 条规则打包为 OPA Bundle，可用于热加载或分发给其他网关。
      </n-alert>
      <n-form :model="bundleForm" label-placement="left" label-width="80">
        <n-form-item label="创建者">
          <n-input v-model:value="bundleForm.created_by" placeholder="如: admin" />
        </n-form-item>
      </n-form>
      <template #action>
        <n-button @click="showBundlePack = false">取消</n-button>
        <n-button type="primary" :loading="packing" @click="handlePackBundleFromModal">打包</n-button>
      </template>
    </n-modal>

    <!-- 编辑器内模拟对话框 -->
    <n-modal v-model:show="showSimulateForEditor" preset="dialog" title="策略模拟测试" style="width: 700px">
      <n-alert type="info" :bordered="false" style="margin-bottom: 16px">
        使用当前编辑器中的规则进行模拟测试。请填写模拟请求参数。
      </n-alert>
      <n-form :model="simulateInput" label-placement="left" label-width="120" size="small">
        <n-grid :cols="2" :x-gap="12">
          <n-grid-item>
            <n-form-item label="SPIFFE ID">
              <n-input v-model:value="simulateInput.device_spiffe_id" />
            </n-form-item>
          </n-grid-item>
          <n-grid-item>
            <n-form-item label="协议">
              <n-select v-model:value="simulateInput.protocol" :options="protocolOptions" />
            </n-form-item>
          </n-grid-item>
          <n-grid-item>
            <n-form-item label="操作">
              <n-select v-model:value="simulateInput.operation" :options="operationOptions" />
            </n-form-item>
          </n-grid-item>
          <n-grid-item>
            <n-form-item label="目标地址">
              <n-input v-model:value="simulateInput.target" />
            </n-form-item>
          </n-grid-item>
          <n-grid-item>
            <n-form-item label="功能码">
              <n-input-number v-model:value="simulateInput.function_code" :min="0" :max="127" style="width: 100%" />
            </n-form-item>
          </n-grid-item>
          <n-grid-item>
            <n-form-item label="MQTT Topic">
              <n-input v-model:value="simulateInput.topic" />
            </n-form-item>
          </n-grid-item>
          <n-grid-item>
            <n-form-item label="行为评分">
              <n-input-number v-model:value="simulateInput.behavior_score" :min="0" :max="1" :step="0.01" style="width: 100%" />
            </n-form-item>
          </n-grid-item>
          <n-grid-item>
            <n-form-item label="维护模式">
              <n-switch v-model:value="simulateInput.maintenance_mode" />
            </n-form-item>
          </n-grid-item>
        </n-grid>
      </n-form>
      <template #action>
        <n-button @click="showSimulateForEditor = false">取消</n-button>
        <n-button type="primary" :loading="simulating" @click="handleSimulateFromEditor">
          <template #icon><n-icon><FlashOutline /></n-icon></template>
          执行模拟
        </n-button>
      </template>
    </n-modal>

    <!-- 模拟结果弹窗（编辑器触发） -->
    <n-modal v-model:show="showSimResult" preset="dialog" title="模拟结果" style="width: 500px">
      <div v-if="simulateResult" style="padding: 8px 0">
        <n-space vertical :size="12">
          <div style="text-align: center; padding: 16px">
            <n-tag
              :type="simulateResult.decision === 'allow' ? 'success' : simulateResult.decision === 'deny' ? 'error' : 'warning'"
              size="large"
              style="font-size: 1.2rem; padding: 8px 24px"
            >
              {{ decisionLabel(simulateResult.decision) }}
            </n-tag>
          </div>
          <n-divider />
          <div>
            <strong>匹配原因：</strong>{{ simulateResult.reason }}
          </div>
          <div>
            <strong>匹配规则：</strong>
            <n-space style="margin-top: 4px">
              <n-tag v-for="rule in simulateResult.matched_rules" :key="rule" type="info" size="small">
                {{ rule }}
              </n-tag>
              <span v-if="!simulateResult.matched_rules?.length" style="color: #86909C">无匹配</span>
            </n-space>
          </div>
        </n-space>
      </div>
      <template #action>
        <n-button @click="showSimResult = false">关闭</n-button>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useMessage } from 'naive-ui'
import {
  RefreshOutline, CubeOutline, CodeSlashOutline, AddOutline, TrashOutline,
  FlashOutline, DocumentTextOutline, CloudDownloadOutline, SwapHorizontalOutline,
} from '@vicons/ionicons5'
import { opaApi } from '@/api'

const message = useMessage()

// === 决策中文标签 ===
function decisionLabel(decision: string): string {
  const map: Record<string, string> = {
    'allow': '允许 (ALLOW)',
    'deny': '拒绝 (DENY)',
    'log': '记录 (LOG)',
    'redirect': '重定向 (REDIRECT)',
  }
  return map[decision?.toLowerCase()] || decision?.toUpperCase() || '-'
}

// === Tab 状态 ===
const activeTab = ref('editor')

// === 引擎版本状态 ===
const revision = ref<any>(null)
const revLoading = ref(false)

// === 规则编辑器 ===
const rules = ref<any[]>([])

const decisionOptions = [
  { label: '允许 (allow)', value: 'allow' },
  { label: '拒绝 (deny)', value: 'deny' },
  { label: '记录 (log)', value: 'log' },
  { label: '重定向 (redirect)', value: 'redirect' },
]

// === 模拟器 ===
const simulating = ref(false)
const simulateResult = ref<any>(null)
const showSimulateForEditor = ref(false)
const showSimResult = ref(false)

const protocolOptions = [
  { label: 'Modbus TCP', value: 'modbus' },
  { label: 'MQTT', value: 'mqtt' },
  { label: 'HTTP', value: 'http' },
  { label: 'OPC UA', value: 'opcua' },
]

const operationOptions = [
  { label: '读 (read)', value: 'read' },
  { label: '写 (write)', value: 'write' },
  { label: '发布 (publish)', value: 'publish' },
  { label: '订阅 (subscribe)', value: 'subscribe' },
]

const simulateInput = ref({
  device_spiffe_id: 'spiffe://iot-ztna.acme.com/device/SN-A001',
  protocol: 'modbus',
  operation: 'read',
  target: '192.168.1.100:502',
  function_code: 3,
  topic: '',
  behavior_score: 0.1,
  maintenance_mode: false,
})

// === Bundle 管理 ===
const showBundlePack = ref(false)
const showBundle = ref(false)
const packing = ref(false)
const hotLoading = ref(false)
const syncing = ref(false)
const hotLoadRevision = ref('')
const bundleForm = ref({ created_by: 'admin' })
const bundleHistory = ref<any[]>([])

const bundleColumns = [
  { title: 'Revision', key: 'revision', width: 280 },
  { title: 'Checksum', key: 'checksum', width: 200, ellipsis: { tooltip: true } },
  { title: '大小', key: 'size', width: 120 },
  { title: '创建时间', key: 'created_at', width: 180, render: (row: any) => row.created_at ? new Date(row.created_at).toLocaleString() : '-' },
]

// === 语法帮助 ===
const helpCode1 = `allow if input.protocol == "modbus"
deny if input.protocol != "mqtt"`
const helpCode2 = `allow if input.protocol == "mqtt" and input.operation == "publish" and input.topic matches "^sensors/.+/telemetry$"`
const helpCode3 = `deny if input.topic matches "^untrusted/.*"`
const helpCode4 = `allow if input.function_code in [3, 4, 1, 2]`
const helpCode5 = `deny
# 无条件 deny，优先级设为 0 作为兜底规则`

// === 方法 ===

async function loadRevision() {
  revLoading.value = true
  try {
    const res = await opaApi.revision()
    if (res.success && res.data) {
      revision.value = res.data
    }
  } catch (e: any) {
    message.error(e.message)
  } finally {
    revLoading.value = false
  }
}

async function loadDefaultRules() {
  try {
    const res = await opaApi.defaultRules()
    if (res.success && res.data) {
      rules.value = res.data
      message.success(`已加载 ${rules.value.length} 条默认规则`)
    }
  } catch (e: any) {
    message.error(e.message)
  }
}

function addRule() {
  rules.value.push({
    name: `规则-${rules.value.length + 1}`,
    description: '',
    rego: 'deny',
    decision: 'deny',
    priority: 0,
  })
}

function removeRule(index: number) {
  rules.value.splice(index, 1)
}

async function handleSimulate() {
  simulating.value = true
  simulateResult.value = null
  try {
    const payload = {
      ...simulateInput.value,
      function_code: simulateInput.value.function_code || null,
      topic: simulateInput.value.topic || null,
      behavior_score: simulateInput.value.behavior_score ?? null,
    }
    const res = await opaApi.simulate(payload)
    if (res.success && res.data) {
      simulateResult.value = res.data
    }
  } catch (e: any) {
    message.error(`模拟失败: ${e.message}`)
  } finally {
    simulating.value = false
  }
}

async function handleSimulateFromEditor() {
  simulating.value = true
  try {
    const payload = {
      ...simulateInput.value,
      function_code: simulateInput.value.function_code || null,
      topic: simulateInput.value.topic || null,
      behavior_score: simulateInput.value.behavior_score ?? null,
    }
    const res = await opaApi.simulate(payload)
    if (res.success && res.data) {
      simulateResult.value = res.data
      showSimulateForEditor.value = false
      showSimResult.value = true
    }
  } catch (e: any) {
    message.error(`模拟失败: ${e.message}`)
  } finally {
    simulating.value = false
  }
}

async function handlePackBundle() {
  if (rules.value.length === 0) {
    message.warning('请先添加或加载规则')
    return
  }
  showBundlePack.value = true
}

async function handlePackBundleFromModal() {
  if (rules.value.length === 0) {
    message.warning('没有可打包的规则')
    return
  }
  packing.value = true
  try {
    const res = await opaApi.packBundle({
      rules: rules.value,
      created_by: bundleForm.value.created_by,
    })
    if (res.success && res.data) {
      message.success(`Bundle 打包成功! Revision: ${res.data.revision?.substring(0, 8)}..., 规则数: ${res.data.rule_count}`)
      hotLoadRevision.value = res.data.revision
      bundleHistory.value.unshift({
        revision: res.data.revision,
        checksum: res.data.checksum,
        size: res.data.size,
        created_at: new Date().toISOString(),
      })
      showBundlePack.value = false
      activeTab.value = 'bundle'
      loadRevision()
    }
  } catch (e: any) {
    message.error(`打包失败: ${e.message}`)
  } finally {
    packing.value = false
  }
}

async function handleHotLoad() {
  if (!hotLoadRevision.value) {
    message.warning('请输入 Bundle revision')
    return
  }
  hotLoading.value = true
  try {
    const res = await opaApi.hotLoad(hotLoadRevision.value)
    if (res.success && res.data) {
      message.success(`Bundle 热加载成功! ${res.data.message}`)
      loadRevision()
    }
  } catch (e: any) {
    message.error(`热加载失败: ${e.message}`)
  } finally {
    hotLoading.value = false
  }
}

async function handleSyncBundle() {
  syncing.value = true
  try {
    const res = await opaApi.sync()
    if (res.success && res.data) {
      message.success(`默认策略已重载! ${res.data.message}`)
      loadRevision()
    }
  } catch (e: any) {
    message.error(`同步失败: ${e.message}`)
  } finally {
    syncing.value = false
  }
}

// === 初始化 ===
onMounted(async () => {
  await loadRevision()
  await loadDefaultRules()
})
</script>

<style scoped>
.rule-item {
  margin-bottom: 0;
}

.simulate-result {
  padding: 8px 0;
}

.result-label {
  font-weight: 600;
  color: #1D2129;
}

.result-value {
  color: #4E5969;
}

.help-card {
  background: #FAFBFC;
}

:deep(.n-code) {
  font-size: 13px;
}
</style>
