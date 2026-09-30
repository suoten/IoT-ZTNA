<template>
  <div>
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px">
      <h2 class="page-title">策略管理</h2>
      <n-space>
        <n-button @click="showTemplates = true">
          <template #icon><n-icon><LibraryOutline /></n-icon></template>
          策略模板库
        </n-button>
        <n-button type="primary" @click="openCreate">
          <template #icon><n-icon><AddOutline /></n-icon></template>
          创建策略
        </n-button>
      </n-space>
    </div>

    <!-- 引导提示 -->
    <n-alert type="info" style="margin-bottom: 16px" :bordered="false">
      <template #header>
        <span style="font-size: 0.85rem">💡 新手指南</span>
      </template>
      <div style="font-size: 0.8rem; color: #4E5969; line-height: 1.8">
        策略用于控制设备的访问权限。你可以：
        <br />
        1. 点击「策略模板库」选择常用策略模板，一键加载
        <br />
        2. 点击「创建策略」自定义规则，支持 JSON 格式
        <br />
        3. 策略创建后自动发布生效，可随时删除重新创建
      </div>
    </n-alert>

    <div class="card">
      <n-data-table :columns="columns" :data="policies" :loading="loading">
        <template #empty>
          <n-empty description="还没有策略">
            <template #icon>
              <n-icon size="48" color="#86909C"><ShieldCheckmarkOutline /></n-icon>
            </template>
            <template #extra>
              <n-space vertical align="center">
                <n-text depth="3" style="font-size: 0.85rem">策略是控制设备访问权限的规则集合</n-text>
                <n-button type="primary" size="small" @click="showTemplates = true">
                  <template #icon><n-icon><LibraryOutline /></n-icon></template>
                  从模板库创建
                </n-button>
                <n-button size="small" @click="openCreate">
                  <template #icon><n-icon><AddOutline /></n-icon></template>
                  自定义创建
                </n-button>
              </n-space>
            </template>
          </n-empty>
        </template>
      </n-data-table>
    </div>

    <!-- 策略模板库对话框 -->
    <n-modal v-model:show="showTemplates" preset="dialog" title="策略模板库" style="width: 800px">
      <n-space vertical :size="12">
        <n-text depth="3" style="font-size: 0.8rem">
          选择一个适合你场景的策略模板，点击「使用此模板」即可加载到创建对话框中。
        </n-text>
        <n-collapse>
          <n-collapse-item
            v-for="(templates, category) in groupedTemplates"
            :key="category"
            :title="category"
            :name="category"
          >
            <template #header-extra>
              <n-tag size="tiny" round>{{ templates.length }} 个模板</n-tag>
            </template>
            <n-space vertical :size="12">
              <div
                v-for="tpl in templates"
                :key="tpl.name"
                style="border: 1px solid #E5E6EB; border-radius: 8px; padding: 12px 16px;"
              >
                <div style="display: flex; justify-content: space-between; align-items: center;">
                  <div>
                    <n-text style="font-weight: 600; font-size: 0.9rem">{{ tpl.name }}</n-text>
                    <n-text depth="3" style="font-size: 0.8rem; margin-left: 8px">{{ tpl.rules.length }} 条规则</n-text>
                  </div>
                  <n-button size="small" type="primary" @click="useTemplate(tpl)">
                    使用此模板
                  </n-button>
                </div>
                <n-text depth="2" style="font-size: 0.8rem; margin-top: 4px; display: block">
                  {{ tpl.description }}
                </n-text>
              </div>
            </n-space>
          </n-collapse-item>
        </n-collapse>
      </n-space>
    </n-modal>

    <!-- 创建策略对话框 -->
    <n-modal v-model:show="showCreate" preset="dialog" title="创建策略" style="width: 700px">
      <n-form :model="formData" label-placement="left" label-width="80">
        <n-form-item label="名称">
          <n-input v-model:value="formData.name" placeholder="策略名称，如：生产环境综合策略" />
        </n-form-item>
        <n-form-item label="创建者">
          <n-input v-model:value="formData.created_by" placeholder="如: admin" />
        </n-form-item>
        <n-form-item label="规则 (JSON)">
          <n-input v-model:value="formData.rules_json" type="textarea" :rows="15" placeholder="策略规则 JSON" />
        </n-form-item>
        <n-space>
          <n-button size="small" @click="loadDefaultRules">加载默认模板</n-button>
          <n-button size="small" @click="showTemplates = true">从模板库选择</n-button>
        </n-space>
      </n-form>
      <template #action>
        <n-button @click="showCreate = false">取消</n-button>
        <n-button type="primary" :loading="creating" @click="handleCreate">创建并发布</n-button>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, h } from 'vue'
import { NTag, NButton, NSpace, NIcon, NAlert, NCollapse, NCollapseItem, NText, useMessage } from 'naive-ui'
import { AddOutline, ShieldCheckmarkOutline, LibraryOutline } from '@vicons/ionicons5'
import { policyApi, type Policy } from '@/api'

const message = useMessage()
const loading = ref(false)
const policies = ref<Policy[]>([])
const showCreate = ref(false)
const showTemplates = ref(false)
const creating = ref(false)

const formData = ref({
  name: '',
  created_by: 'admin',
  rules_json: '[]',
})

const templates = ref<any[]>([])
const groupedTemplates = computed(() => {
  const groups: Record<string, any[]> = {}
  for (const tpl of templates.value) {
    if (!groups[tpl.category]) groups[tpl.category] = []
    groups[tpl.category].push(tpl)
  }
  return groups
})

const columns = [
  { title: '名称', key: 'name', width: 200 },
  { title: '版本', key: 'version', width: 80 },
  {
    title: '状态',
    key: 'status',
    width: 100,
    render: (row: Policy) => {
      const type = row.status === 'published' ? 'success' : row.status === 'archived' ? 'default' : 'warning'
      const label = row.status === 'published' ? '已发布' : row.status === 'archived' ? '已归档' : row.status === 'draft' ? '草稿' : row.status
      return h(NTag, { type, size: 'small' }, () => label)
    },
  },
  { title: '创建者', key: 'created_by', width: 100 },
  {
    title: '创建时间',
    key: 'created_at',
    width: 180,
    render: (row: Policy) => new Date(row.created_at).toLocaleString(),
  },
  {
    title: '操作',
    key: 'actions',
    width: 200,
    render: (row: Policy) =>
      h(NSpace, {}, () => [
        h(NButton, {
          size: 'small',
          type: 'success',
          disabled: row.status === 'published',
          onClick: () => handlePublish(row),
        }, () => '发布'),
        h(NButton, {
          size: 'small',
          type: 'error',
          onClick: () => handleDelete(row),
        }, () => '删除'),
      ]),
  },
]

function openCreate() {
  formData.value = { name: '', created_by: 'admin', rules_json: '[]' }
  showCreate.value = true
}

async function loadData() {
  loading.value = true
  try {
    const res = await policyApi.list(1, 100)
    if (res.success && res.data) {
      policies.value = res.data.items || []
    }
  } catch (e: any) {
    message.error(e.message)
  } finally {
    loading.value = false
  }
}

async function loadTemplates() {
  try {
    const res = await policyApi.templates()
    if (res.success && res.data) {
      templates.value = res.data
    }
  } catch (e: any) {
    message.error(e.message)
  }
}

async function loadDefaultRules() {
  try {
    const res = await policyApi.defaultRules()
    if (res.success && res.data) {
      formData.value.rules_json = JSON.stringify(res.data, null, 2)
      message.success('已加载默认模板')
    }
  } catch (e: any) {
    message.error(e.message)
  }
}

function useTemplate(tpl: any) {
  formData.value.name = tpl.name
  formData.value.rules_json = JSON.stringify(tpl.rules, null, 2)
  showTemplates.value = false
  showCreate.value = true
  message.success(`已加载模板: ${tpl.name}`)
}

async function handleCreate() {
  creating.value = true
  try {
    const rules = JSON.parse(formData.value.rules_json)
    const res = await policyApi.create({
      name: formData.value.name,
      rules,
      created_by: formData.value.created_by,
    })
    if (res.success && res.data) {
      await policyApi.publish(res.data)
      message.success('策略创建并发布成功')
      showCreate.value = false
      loadData()
    }
  } catch (e: any) {
    message.error(`创建失败: ${e.message}`)
  } finally {
    creating.value = false
  }
}

async function handlePublish(row: Policy) {
  try {
    await policyApi.publish(row.id)
    message.success('策略已发布')
    loadData()
  } catch (e: any) {
    message.error(e.message)
  }
}

async function handleDelete(row: Policy) {
  const confirmed = window.confirm(`确定要删除策略 "${row.name}" 吗？`)
  if (!confirmed) return
  try {
    await policyApi.delete(row.id)
    message.success('策略已删除')
    loadData()
  } catch (e: any) {
    message.error(e.message)
  }
}

onMounted(() => {
  loadData()
  loadTemplates()
})
</script>
