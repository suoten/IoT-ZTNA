<template>
  <div>
    <div class="page-header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px">
      <h2 class="page-title">用户管理</h2>
      <n-button type="primary" @click="showCreate = true">
        <template #icon><n-icon><PersonAddOutline /></n-icon></template>
        新增用户
      </n-button>
    </div>

    <!-- 用户列表 -->
    <div class="card">
      <n-data-table
        :columns="columns"
        :data="users"
        :loading="loading"
        :bordered="false"
        :row-key="(row: any) => row.id"
      />
    </div>

    <!-- 新增用户对话框 -->
    <n-modal v-model:show="showCreate" preset="dialog" title="新增用户" :show-icon="false" style="width: 480px">
      <n-form ref="createFormRef" :model="createForm" :rules="createRules" label-placement="top">
        <n-form-item label="用户名" path="username">
          <n-input v-model:value="createForm.username" placeholder="3-32 个字符" />
        </n-form-item>
        <n-form-item label密码 path="password">
          <n-input v-model:value="createForm.password" type="password" placeholder="至少 6 个字符" show-password-on="click" />
        </n-form-item>
        <n-form-item label="角色" path="role">
          <n-select v-model:value="createForm.role" :options="roleOptions" />
        </n-form-item>
      </n-form>
      <template #action>
        <n-button @click="showCreate = false">取消</n-button>
        <n-button type="primary" :loading="creating" @click="handleCreate">确认创建</n-button>
      </template>
    </n-modal>

    <!-- 修改密码对话框 -->
    <n-modal v-model:show="showChangePassword" preset="dialog" title="修改密码" :show-icon="false" style="width: 480px">
      <n-form ref="changePwdFormRef" :model="changePwdForm" :rules="changePwdRules" label-placement="top">
        <n-form-item label="旧密码" path="old_password">
          <n-input v-model:value="changePwdForm.old_password" type="password" placeholder="请输入当前密码" show-password-on="click" />
        </n-form-item>
        <n-form-item label="新密码" path="new_password">
          <n-input v-model:value="changePwdForm.new_password" type="password" placeholder="至少 6 个字符" show-password-on="click" />
        </n-form-item>
        <n-form-item label="确认新密码" path="confirm_password">
          <n-input v-model:value="changePwdForm.confirm_password" type="password" placeholder="再次输入新密码" show-password-on="click" />
        </n-form-item>
      </n-form>
      <template #action>
        <n-button @click="showChangePassword = false">取消</n-button>
        <n-button type="primary" :loading="changingPwd" @click="handleChangePassword">确认修改</n-button>
      </template>
    </n-modal>

    <!-- 重置密码对话框 -->
    <n-modal v-model:show="showResetPassword" preset="dialog" :title="`重置密码 - ${resetPwdTarget?.username || ''}`" :show-icon="false" style="width: 480px">
      <n-form ref="resetPwdFormRef" :model="resetPwdForm" :rules="resetPwdRules" label-placement="top">
        <n-form-item label="新密码" path="new_password">
          <n-input v-model:value="resetPwdForm.new_password" type="password" placeholder="至少 6 个字符" show-password-on="click" />
        </n-form-item>
        <n-form-item label="确认新密码" path="confirm_password">
          <n-input v-model:value="resetPwdForm.confirm_password" type="password" placeholder="再次输入新密码" show-password-on="click" />
        </n-form-item>
      </n-form>
      <template #action>
        <n-button @click="showResetPassword = false">取消</n-button>
        <n-button type="warning" :loading="resettingPwd" @click="handleResetPassword">确认重置</n-button>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, h, onMounted } from 'vue'
import { NIcon, NButton, NTag, NSpace, useMessage, type DataTableColumns, type FormInst, type FormRules } from 'naive-ui'
import {
  PersonAddOutline,
  CreateOutline,
  KeyOutline,
  TrashOutline,
} from '@vicons/ionicons5'
import api from '@/api'

interface User {
  id: string
  username: string
  role: string
  status: string
  created_at: string
  last_login: string | null
}

const message = useMessage()
const loading = ref(false)
const users = ref<User[]>([])

const roleOptions = [
  { label: '管理员 (Admin) - 全部权限', value: 'admin' },
  { label: '操作员 (Operator) - 设备/策略/证书管理', value: 'operator' },
  { label: '审计员 (Auditor) - 只读', value: 'auditor' },
]

const roleTagType = (role: string) => {
  switch (role) {
    case 'admin': return 'error'
    case 'operator': return 'warning'
    case 'auditor': return 'info'
    default: return 'default'
  }
}

const roleLabel = (role: string) => {
  switch (role) {
    case 'admin': return '管理员'
    case 'operator': return '操作员'
    case 'auditor': return '审计员'
    default: return role
  }
}

const columns: DataTableColumns<User> = [
  { title: '用户名', key: 'username', width: 150 },
  {
    title: '角色',
    key: 'role',
    width: 120,
    render: (row) => h(NTag, { type: roleTagType(row.role) as any, size: 'small' }, { default: () => roleLabel(row.role) }),
  },
  {
    title: '状态',
    key: 'status',
    width: 100,
    render: (row) => h(NTag, { type: row.status === 'active' ? 'success' : 'default', size: 'small' }, { default: () => row.status === 'active' ? '正常' : '已禁用' }),
  },
  {
    title: '创建时间',
    key: 'created_at',
    width: 180,
    render: (row) => new Date(row.created_at).toLocaleString('zh-CN'),
  },
  {
    title: '最后登录',
    key: 'last_login',
    width: 180,
    render: (row) => row.last_login ? new Date(row.last_login).toLocaleString('zh-CN') : '从未登录',
  },
  {
    title: '操作',
    key: 'actions',
    width: 200,
    render: (row) => {
      return h(NSpace, { size: 'small' }, {
        default: () => [
          h(NButton, {
            size: 'small',
            quaternary: true,
            type: 'primary',
            onClick: () => openChangePassword(row),
          }, { icon: () => h(NIcon, null, { default: () => h(KeyOutline) }), default: () => '改密' }),
          h(NButton, {
            size: 'small',
            quaternary: true,
            type: 'warning',
            onClick: () => toggleUserStatus(row),
          }, { default: () => row.status === 'active' ? '禁用' : '启用' }),
          h(NButton, {
            size: 'small',
            quaternary: true,
            type: 'error',
            disabled: row.role === 'admin' && users.value.filter(u => u.role === 'admin' && u.status === 'active').length <= 1,
            onClick: () => handleDelete(row),
          }, { icon: () => h(NIcon, null, { default: () => h(TrashOutline) }) }),
        ],
      })
    },
  },
]

// === 新增用户 ===
const showCreate = ref(false)
const creating = ref(false)
const createFormRef = ref<FormInst | null>(null)
const createForm = ref({
  username: '',
  password: '',
  role: 'operator',
})
const createRules: FormRules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    { min: 3, max: 32, message: '用户名长度 3-32 个字符', trigger: 'blur' },
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 6, message: '密码至少 6 个字符', trigger: 'blur' },
  ],
  role: [{ required: true, message: '请选择角色', trigger: 'change' }],
}

async function handleCreate() {
  try {
    await createFormRef.value?.validate()
  } catch {
    return
  }
  creating.value = true
  try {
    await api.post('/users', {
      username: createForm.value.username,
      password: createForm.value.password,
      role: createForm.value.role,
    })
    message.success('用户创建成功')
    showCreate.value = false
    createForm.value = { username: '', password: '', role: 'operator' }
    await loadData()
  } catch (e: any) {
    message.error('创建失败: ' + e.message)
  } finally {
    creating.value = false
  }
}

// === 修改密码（自己的密码） ===
const showChangePassword = ref(false)
const changingPwd = ref(false)
const changePwdFormRef = ref<FormInst | null>(null)
const changePwdTarget = ref<User | null>(null)
const changePwdForm = ref({
  old_password: '',
  new_password: '',
  confirm_password: '',
})
const changePwdRules: FormRules = {
  old_password: [{ required: true, message: '请输入旧密码', trigger: 'blur' }],
  new_password: [
    { required: true, message: '请输入新密码', trigger: 'blur' },
    { min: 6, message: '密码至少 6 个字符', trigger: 'blur' },
  ],
  confirm_password: [
    { required: true, message: '请确认新密码', trigger: 'blur' },
    {
      validator: (_rule: any, value: string) => value === changePwdForm.value.new_password,
      message: '两次输入的密码不一致',
      trigger: 'blur',
    },
  ],
}

function openChangePassword(row: User) {
  changePwdTarget.value = row
  changePwdForm.value = { old_password: '', new_password: '', confirm_password: '' }
  showChangePassword.value = true
}

async function handleChangePassword() {
  try {
    await changePwdFormRef.value?.validate()
  } catch {
    return
  }
  if (!changePwdTarget.value) return
  changingPwd.value = true
  try {
    await api.post(`/users/${changePwdTarget.value.id}/password`, {
      old_password: changePwdForm.value.old_password,
      new_password: changePwdForm.value.new_password,
    })
    message.success('密码修改成功')
    showChangePassword.value = false
  } catch (e: any) {
    message.error('修改失败: ' + e.message)
  } finally {
    changingPwd.value = false
  }
}

// === 重置密码（管理员） ===
const showResetPassword = ref(false)
const resettingPwd = ref(false)
const resetPwdFormRef = ref<FormInst | null>(null)
const resetPwdTarget = ref<User | null>(null)
const resetPwdForm = ref({
  new_password: '',
  confirm_password: '',
})
const resetPwdRules: FormRules = {
  new_password: [
    { required: true, message: '请输入新密码', trigger: 'blur' },
    { min: 6, message: '密码至少 6 个字符', trigger: 'blur' },
  ],
  confirm_password: [
    { required: true, message: '请确认新密码', trigger: 'blur' },
    {
      validator: (_rule: any, value: string) => value === resetPwdForm.value.new_password,
      message: '两次输入的密码不一致',
      trigger: 'blur',
    },
  ],
}

async function handleResetPassword() {
  try {
    await resetPwdFormRef.value?.validate()
  } catch {
    return
  }
  if (!resetPwdTarget.value) return
  resettingPwd.value = true
  try {
    await api.post(`/users/${resetPwdTarget.value.id}/reset-password`, {
      new_password: resetPwdForm.value.new_password,
    })
    message.success('密码已重置')
    showResetPassword.value = false
  } catch (e: any) {
    message.error('重置失败: ' + e.message)
  } finally {
    resettingPwd.value = false
  }
}

// === 启用/禁用用户 ===
async function toggleUserStatus(row: User) {
  const newStatus = row.status === 'active' ? 'disabled' : 'active'
  const action = newStatus === 'disabled' ? '禁用' : '启用'
  if (!window.confirm(`确定要${action}用户 "${row.username}" 吗？`)) return
  try {
    await api.put(`/users/${row.id}`, { status: newStatus })
    message.success(`已${action}用户`)
    await loadData()
  } catch (e: any) {
    message.error(`${action}失败: ` + e.message)
  }
}

// === 删除用户 ===
async function handleDelete(row: User) {
  if (!window.confirm(`确定要删除用户 "${row.username}" 吗？此操作不可恢复！`)) return
  try {
    await api.delete(`/users/${row.id}`)
    message.success('用户已删除')
    await loadData()
  } catch (e: any) {
    message.error('删除失败: ' + e.message)
  }
}

// === 加载数据 ===
async function loadData() {
  loading.value = true
  try {
    const res = await api.get<{ users: User[]; total: number }>('/users')
    if (res.success && res.data) {
      users.value = res.data.users
    }
  } catch (e: any) {
    message.error('加载用户列表失败: ' + e.message)
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadData()
})
</script>
