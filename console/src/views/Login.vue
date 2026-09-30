<template>
  <div class="login-container">
    <div class="login-card">
      <!-- Logo 区 -->
      <div class="login-header">
        <div class="login-logo">
          <n-icon :size="40" color="#165DFF">
            <ShieldCheckmarkOutline />
          </n-icon>
        </div>
        <h1 class="login-title">IoT-ZTNA</h1>
        <p class="login-subtitle">零信任网络访问网关</p>
      </div>

      <!-- 登录表单 -->
      <n-form ref="formRef" :model="formData" :rules="rules" label-placement="top" size="large">
        <n-form-item label="用户名" path="username">
          <n-input
            v-model:value="formData.username"
            placeholder="请输入用户名"
            :input-props="{ autocomplete: 'username' }"
            @keyup.enter="handleLogin"
          >
            <template #prefix>
              <n-icon color="#86909C"><PersonOutline /></n-icon>
            </template>
          </n-input>
        </n-form-item>
        <n-form-item label="密码" path="password">
          <n-input
            v-model:value="formData.password"
            type="password"
            show-password-on="click"
            placeholder="请输入密码"
            :input-props="{ autocomplete: 'current-password' }"
            @keyup.enter="handleLogin"
          >
            <template #prefix>
              <n-icon color="#86909C"><LockClosedOutline /></n-icon>
            </template>
          </n-input>
        </n-form-item>

        <n-button
          type="primary"
          block
          size="large"
          :loading="loading"
          style="margin-top: 8px"
          @click="handleLogin"
        >
          <template #icon><n-icon><LogInOutline /></n-icon></template>
          登 录
        </n-button>
      </n-form>

      <!-- 底部信息 -->
      <div class="login-footer">
        <span>IoT-ZTNA Gateway v1.0</span>
        <n-divider vertical />
        <span>Apache-2.0</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { NIcon, useMessage, type FormInst, type FormRules } from 'naive-ui'
import {
  ShieldCheckmarkOutline,
  PersonOutline,
  LockClosedOutline,
  LogInOutline,
} from '@vicons/ionicons5'
import { authApi } from '@/api'

const router = useRouter()
const message = useMessage()
const formRef = ref<FormInst | null>(null)
const loading = ref(false)

const formData = reactive({
  username: '',
  password: '',
})

const rules: FormRules = {
  username: { required: true, message: '请输入用户名', trigger: 'blur' },
  password: { required: true, message: '请输入密码', trigger: 'blur' },
}

async function handleLogin() {
  try {
    await formRef.value?.validate()
  } catch {
    return
  }

  loading.value = true
  try {
    const res = await authApi.login(formData.username, formData.password)
    if (res.success && res.data) {
      localStorage.setItem('ztna_token', res.data.token)
      localStorage.setItem('ztna_role', res.data.role)
      localStorage.setItem('ztna_user', formData.username)
      message.success('登录成功')
      router.push('/dashboard')
    }
  } catch (e: any) {
    message.error(e.message || '登录失败')
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.login-container {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #0A0F1E 0%, #1a2332 50%, #16213e 100%);
  position: relative;
  overflow: hidden;
}
.login-container::before {
  content: '';
  position: absolute;
  width: 600px;
  height: 600px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(22, 93, 255, 0.15) 0%, transparent 70%);
  top: -200px;
  right: -200px;
}
.login-container::after {
  content: '';
  position: absolute;
  width: 400px;
  height: 400px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(24, 160, 88, 0.1) 0%, transparent 70%);
  bottom: -100px;
  left: -100px;
}
.login-card {
  width: 420px;
  background: rgba(255, 255, 255, 0.95);
  border-radius: 16px;
  padding: 40px 36px 32px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  backdrop-filter: blur(20px);
  position: relative;
  z-index: 1;
}
.login-header {
  text-align: center;
  margin-bottom: 32px;
}
.login-logo {
  width: 72px;
  height: 72px;
  border-radius: 18px;
  background: linear-gradient(135deg, #165DFF 0%, #4080FF 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 16px;
  box-shadow: 0 8px 24px rgba(22, 93, 255, 0.3);
}
.login-title {
  font-size: 1.6rem;
  font-weight: 700;
  color: #1D2129;
  margin-bottom: 4px;
}
.login-subtitle {
  font-size: 0.9rem;
  color: #86909C;
}
.login-footer {
  text-align: center;
  margin-top: 24px;
  font-size: 0.75rem;
  color: #86909C;
}
</style>
