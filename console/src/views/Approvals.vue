<template>
  <div>
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px">
      <h2 class="page-title">策略审批</h2>
      <n-space>
        <n-button @click="showSubmit = true">
          <template #icon><n-icon><PaperPlaneOutline /></n-icon></template>
          提交审批
        </n-button>
        <n-button @click="loadData" :loading="loading">
          <template #icon><n-icon><RefreshOutline /></n-icon></template>
          刷新
        </n-button>
      </n-space>
    </div>

    <!-- 引导提示 -->
    <n-alert type="info" style="margin-bottom: 16px" :bordered="false">
      <template #header>
        <span style="font-size: 0.85rem">💡 审批流程说明</span>
      </template>
      <div style="font-size: 0.8rem; color: #4E5969; line-height: 1.8">
        审批流程：操作员提交策略 → 管理员审批 → 批准则自动发布生效 → 拒绝则驳回
        <br />
        所有审批记录全量留痕，可在安全审计中查看
      </div>
    </n-alert>

    <div class="card">
      <n-data-table :columns="columns" :data="approvals" :loading="loading">
        <template #empty>
          <n-empty description="暂无审批请求">
            <template #icon>
              <n-icon size="48" color="#86909C"><DocumentTextOutline /></n-icon>
            </template>
            <template #extra>
              <n-space vertical align="center">
                <n-text depth="3" style="font-size: 0.85rem">操作员提交的策略变更需要管理员审批</n-text>
                <n-button size="small" @click="showSubmit = true">
                  <template #icon><n-icon><PaperPlaneOutline /></n-icon></template>
                  提交审批
                </n-button>
              </n-space>
            </template>
          </n-empty>
        </template>
      </n-data-table>
    </div>

    <!-- 审批对话框 -->
    <n-modal v-model:show="showReview" preset="dialog" title="审批策略" style="width: 700px">
      <n-descriptions v-if="reviewTarget" :column="1" label-placement="left">
        <n-descriptions-item label="策略名称">{{ reviewTarget.policy_name }}</n-descriptions-item>
        <n-descriptions-item label="提交者">{{ reviewTarget.submitted_by }}</n-descriptions-item>
        <n-descriptions-item label="提交时间">{{ new Date(reviewTarget.submitted_at).toLocaleString() }}</n-descriptions-item>
        <n-descriptions-item label="策略内容">
          <n-code :code="reviewTarget.rego_content" language="json" />
        </n-descriptions-item>
      </n-descriptions>
      <n-form :model="reviewForm" label-placement="top" style="margin-top: 16px">
        <n-form-item label="审批人">
          <n-input v-model:value="reviewForm.reviewed_by" placeholder="如: admin" />
        </n-form-item>
        <n-form-item label="审批意见">
          <n-input v-model:value="reviewForm.comment" type="textarea" :rows="3" placeholder="审批意见（可选）" />
        </n-form-item>
      </n-form>
      <template #action>
        <n-button @click="showReview = false">取消</n-button>
        <n-button type="error" :loading="reviewing" @click="handleReview(false)">拒绝</n-button>
        <n-button type="success" :loading="reviewing" @click="handleReview(true)">批准并发布</n-button>
      </template>
    </n-modal>

    <!-- 提交审批对话框 -->
    <n-modal v-model:show="showSubmit" preset="dialog" title="提交策略审批" style="width: 700px">
      <n-space vertical :size="12">
        <n-alert type="info" :bordered="false">
          选择一条已有策略提交审批，审批通过后策略会自动发布生效。
        </n-alert>
        <n-form label-placement="left" label-width="80">
          <n-form-item label="提交者">
            <n-input v-model:value="submitForm.submitted_by" placeholder="如: admin" />
          </n-form-item>
          <n-form-item label="选择策略">
            <n-select
              v-model:value="submitForm.policy_id"
              :options="policyOptions"
              placeholder="选择要提交审批的策略"
              @update:value="onPolicySelect"
            />
          </n-form-item>
          <n-form-item label="策略内容">
            <n-input
              v-model:value="submitForm.rego_content"
              type="textarea"
              :rows="10"
              readonly
              placeholder="选择策略后自动填充"
            />
          </n-form-item>
        </n-form>
      </n-space>
      <template #action>
        <n-button @click="showSubmit = false">取消</n-button>
        <n-button type="primary" :loading="submitting" @click="handleSubmit">提交审批</n-button>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, h } from 'vue'
import { NTag, NButton, NSpace, NIcon, NAlert, NDescriptions, NDescriptionsItem, NCode, NSelect, NInput, NForm, NFormItem, NText, useMessage } from 'naive-ui'
import { RefreshOutline, DocumentTextOutline, PaperPlaneOutline } from '@vicons/ionicons5'
import { approvalApi, policyApi, type Policy } from '@/api'

const message = useMessage()
const loading = ref(false)
const reviewing = ref(false)
const submitting = ref(false)
const showReview = ref(false)
const showSubmit = ref(false)
const reviewTarget = ref<any>(null)
const approvals = ref<any[]>([])
const policies = ref<Policy[]>([])

const reviewForm = ref({
  reviewed_by: 'admin',
  comment: '',
})

const submitForm = ref({
  policy_id: '',
  submitted_by: 'admin',
  rego_content: '',
})

const policyOptions = ref<{ label: string; value: string }[]>([])

const columns = [
  { title: '策略名称', key: 'policy_name', width: 200 },
  {
    title: '状态',
    key: 'status',
    width: 100,
    render: (row: any) => {
      const type = row.status === 'pending' ? 'warning' : row.status === 'approved' ? 'success' : row.status === 'rejected' ? 'error' : 'info'
      const label = row.status === 'pending' ? '待审批' : row.status === 'approved' ? '已批准' : row.status === 'rejected' ? '已拒绝' : row.status === 'published' ? '已发布' : row.status
      return h(NTag, { type, size: 'small' }, () => label)
    },
  },
  { title: '提交者', key: 'submitted_by', width: 100 },
  { title: '提交时间', key: 'submitted_at', width: 180, render: (row: any) => new Date(row.submitted_at).toLocaleString() },
  { title: '审批人', key: 'reviewed_by', width: 100, render: (row: any) => row.reviewed_by || '-' },
  { title: '审批时间', key: 'reviewed_at', width: 180, render: (row: any) => row.reviewed_at ? new Date(row.reviewed_at).toLocaleString() : '-' },
  {
    title: '操作',
    key: 'actions',
    width: 150,
    render: (row: any) =>
      row.status === 'pending' ? h(NButton, {
        size: 'small', type: 'primary', onClick: () => openReview(row),
      }, () => '审批') : h('span', { style: 'color: #ccc' }, '-'),
  },
]

async function loadData() {
  loading.value = true
  try {
    const res = await approvalApi.list(1, 100)
    if (res.success && res.data) {
      approvals.value = res.data.items || []
    }
  } catch (e: any) {
    message.error(e.message)
  } finally {
    loading.value = false
  }
}

async function loadPolicies() {
  try {
    const res = await policyApi.list(1, 100)
    if (res.success && res.data) {
      policies.value = res.data.items || []
      policyOptions.value = policies.value.map(p => ({
        label: `${p.name} (v${p.version})`,
        value: p.id,
      }))
    }
  } catch (e: any) {
    message.error(e.message)
  }
}

function onPolicySelect(policyId: string) {
  const policy = policies.value.find(p => p.id === policyId)
  if (policy) {
    submitForm.value.rego_content = policy.rego_content
  }
}

function openReview(row: any) {
  reviewTarget.value = row
  reviewForm.value = { reviewed_by: 'admin', comment: '' }
  showReview.value = true
}

async function handleReview(approved: boolean) {
  if (!reviewTarget.value) return
  reviewing.value = true
  try {
    await approvalApi.review(reviewTarget.value.id, {
      approved,
      reviewed_by: reviewForm.value.reviewed_by,
      comment: reviewForm.value.comment,
    })
    message.success(approved ? '策略已批准并发布' : '策略已被拒绝')
    showReview.value = false
    loadData()
  } catch (e: any) {
    message.error(e.message)
  } finally {
    reviewing.value = false
  }
}

async function handleSubmit() {
  if (!submitForm.value.policy_id) {
    message.warning('请选择要提交审批的策略')
    return
  }
  const policy = policies.value.find(p => p.id === submitForm.value.policy_id)
  if (!policy) return

  submitting.value = true
  try {
    await approvalApi.submit({
      policy_id: policy.id,
      policy_name: policy.name,
      rego_content: policy.rego_content,
      submitted_by: submitForm.value.submitted_by,
    })
    message.success('策略已提交审批，等待管理员审批')
    showSubmit.value = false
    loadData()
  } catch (e: any) {
    message.error(e.message)
  } finally {
    submitting.value = false
  }
}

onMounted(() => {
  loadData()
  loadPolicies()
})
</script>
