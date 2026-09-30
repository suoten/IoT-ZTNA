import axios from 'axios'

// === 类型定义 ===

export interface Device {
  id: string
  spiffe_id: string
  serial_number: string
  model: string
  trust_root: string
  trust_level: string
  status: 'online' | 'offline' | 'revoked' | 'pending'
  capabilities: string[]
  registered_at: string
  last_seen: string | null
  maintenance_mode: boolean
}

export interface Certificate {
  id: string
  spiffe_id: string
  serial_number: string
  pem: string
  issued_at: string
  expires_at: string
  status: 'active' | 'revoked' | 'expired'
}

export interface Policy {
  id: string
  version: string
  name: string
  rego_content: string
  status: 'draft' | 'published' | 'archived'
  created_by: string
  created_at: string
  published_at: string | null
}

export interface Alert {
  id: string
  alert_type: string
  severity: 'info' | 'warning' | 'critical'
  spiffe_id: string | null
  message: string
  details: any
  created_at: string
  acknowledged: boolean
}

export interface AuditLog {
  id: string
  timestamp: string
  actor: string
  action: string
  target: string
  result: string
  details: any
  ip_address: string
}

export interface PagedResponse<T> {
  items: T[]
  total: number
  page: number
  page_size: number
}

export interface ApiResponse<T> {
  success: boolean
  data: T | null
  error: { code: string; message: string } | null
  timestamp: string
}

// === Axios 实例 ===

const instance = axios.create({
  baseURL: '/api/v1',
  timeout: 10000,
  headers: {
    'Content-Type': 'application/json',
  },
})

// 请求拦截器：自动携带 JWT token
instance.interceptors.request.use(
  (config) => {
    const token = localStorage.getItem('ztna_token')
    if (token) {
      config.headers.Authorization = `Bearer ${token}`
    }
    return config
  },
  (error) => Promise.reject(error)
)

// 响应拦截器：解包 response.data，返回 ApiResponse<T>
// 注意：由于 axios 类型约束，拦截器返回类型必须声明为 AxiosResponse，
// 但运行时实际返回的是 response.data（即 ApiResponse<T>）。
// 包装函数 get/post/put/delete 负责将类型断言为正确的 Promise<ApiResponse<T>>
instance.interceptors.response.use(
  (response) => {
    const data = response.data as ApiResponse<unknown>
    if (data && data.success === false) {
      return Promise.reject(new Error(data.error?.message || 'Unknown error'))
    }
    return data as unknown as typeof response
  },
  (error) => {
    // 401 未授权：token 过期或无效，跳转登录页
    if (error.response?.status === 401) {
      localStorage.removeItem('ztna_token')
      localStorage.removeItem('ztna_role')
      localStorage.removeItem('ztna_user')
      // 避免在登录页重复跳转
      if (!window.location.pathname.startsWith('/login')) {
        window.location.href = '/login'
      }
    }
    const msg = error.response?.data?.error?.message || error.message || '网络错误'
    console.error('API Error:', msg)
    return Promise.reject(new Error(msg))
  }
)

// === 类型安全的请求包装 ===
// 由于拦截器已经解包，我们用包装函数让 TypeScript 正确推断返回类型

async function get<T>(url: string, config?: any): Promise<ApiResponse<T>> {
  return instance.get(url, config) as unknown as Promise<ApiResponse<T>>
}

async function post<T>(url: string, data?: any, config?: any): Promise<ApiResponse<T>> {
  return instance.post(url, data, config) as unknown as Promise<ApiResponse<T>>
}

async function put<T>(url: string, data?: any, config?: any): Promise<ApiResponse<T>> {
  return instance.put(url, data, config) as unknown as Promise<ApiResponse<T>>
}

async function del<T>(url: string, config?: any): Promise<ApiResponse<T>> {
  return instance.delete(url, config) as unknown as Promise<ApiResponse<T>>
}

// 通用 API 对象，用于直接调用
const api = { get, post, put, delete: del }
export default api

// === API 方法 ===

export const deviceApi = {
  list: (page = 1, pageSize = 20) =>
    get<PagedResponse<Device>>(`/devices?page=${page}&page_size=${pageSize}`),
  get: (id: string) => get<Device>(`/devices/${id}`),
  register: (data: any) => post<Device>('/devices', data),
  revoke: (id: string) => post<any>(`/devices/${id}/revoke`),
  reactivate: (id: string) => post<any>(`/devices/${id}/reactivate`),
  setMaintenance: (id: string, enabled: boolean) =>
    post<any>(`/devices/${id}/maintenance`, { enabled }),
  delete: (id: string) => del<any>(`/devices/${id}`),
}

export const certApi = {
  list: (page = 1, pageSize = 20) =>
    get<PagedResponse<Certificate>>(`/certificates?page=${page}&page_size=${pageSize}`),
  get: (id: string) => get<Certificate>(`/certificates/${id}`),
  renew: (id: string) => post<any>(`/certificates/${id}/renew`),
  revoke: (id: string) => post<any>(`/certificates/${id}/revoke`),
  crl: () => get<any>('/certificates/crl'),
  checkRevocation: (serialNumber: string) => get<any>(`/certificates/revocation-check?serial_number=${serialNumber}`),
}

export const policyApi = {
  list: (page = 1, pageSize = 20) =>
    get<PagedResponse<Policy>>(`/policies?page=${page}&page_size=${pageSize}`),
  get: (id: string) => get<Policy>(`/policies/${id}`),
  create: (data: any) => post<string>('/policies', data),
  update: (id: string, data: any) => put<any>(`/policies/${id}`, data),
  publish: (id: string) => post<any>(`/policies/${id}/publish`),
  simulate: (id: string, data: any) => post<any>(`/policies/${id}/simulate`, data),
  delete: (id: string) => del<any>(`/policies/${id}`),
  defaultRules: () => get<any>('/policies/default-rules'),
  templates: () => get<any>('/policies/templates'),
  versions: (id: string) => get<any>(`/policies/${id}/versions`),
  rollback: (id: string, versionId: string) =>
    post<any>(`/policies/${id}/rollback/${versionId}`),
}

export const alertApi = {
  list: (page = 1, pageSize = 20) =>
    get<PagedResponse<Alert>>(`/alerts?page=${page}&page_size=${pageSize}`),
  ack: (id: string) => post<any>(`/alerts/${id}/ack`),
}

export const auditApi = {
  list: (page = 1, pageSize = 50) =>
    get<PagedResponse<AuditLog>>(`/audit/logs?page=${page}&page_size=${pageSize}`),
  complianceReport: () => get<any>('/audit/compliance-report'),
  verifyIntegrity: () => get<any>('/audit/verify'),
}

export const authApi = {
  login: (username: string, password: string) =>
    post<{ token: string; expires_in: number; role: string }>('/auth/login', { username, password }),
}

export const integrationApi = {
  receiveDeviceEvent: (data: any) => post<any>('/integrations/device-events', data),
  syncDevices: () => post<any>('/integrations/sync-devices'),
}

export const healthApi = {
  check: () => get<any>('/health'),
}

export const behaviorApi = {
  assess: (spiffeId: string) => get<any>(`/behavior/${encodeURIComponent(spiffeId)}/assess`),
  stats: () => get<any>('/behavior/stats'),
  getBaseline: (spiffeId: string) => get<any>(`/behavior/${encodeURIComponent(spiffeId)}/baseline`),
  updateBaseline: (spiffeId: string) => post<any>(`/behavior/${encodeURIComponent(spiffeId)}/update-baseline`),
}

export const caApi = {
  downloadCert: () => '/ca/cert',
  regenerate: () => post<any>('/ca/regenerate'),
  backup: () => post<any>('/ca/backup'),
  listBackups: () => get<any[]>('/ca/backups'),
  downloadBackupUrl: (filename: string) => `/ca/backups/${filename}`,
}

// === 策略审批 API ===
export const approvalApi = {
  list: (page = 1, pageSize = 20) =>
    get<any>(`/approvals?page=${page}&page_size=${pageSize}`),
  listPending: () => get<any[]>('/approvals/pending'),
  get: (id: string) => get<any>(`/approvals/${id}`),
  submit: (data: any) => post<string>('/approvals', data),
  review: (id: string, data: { approved: boolean; reviewed_by: string; comment: string }) =>
    post<any>(`/approvals/${id}/review`, data),
}

// === 模型管理 API ===
export const modelApi = {
  getInfo: () => get<any>('/models/info'),
  listVersions: () => get<any[]>('/models/versions'),
  train: (data: any) => post<any>('/models/train', data),
  fineTune: (data: any) => post<any>('/models/fine-tune', data),
  activate: (id: string) => post<any>(`/models/${id}/activate`),
  rollback: (id: string) => post<any>(`/models/${id}/rollback`),
  delete: (id: string) => del<any>(`/models/${id}`),
  export: (id: string) => get<any>(`/models/${id}/export`),
}

// === 数据面 API ===
export const dataplaneApi = {
  stats: () => get<any>('/dataplane/stats'),
  clearAll: () => post<any>('/dataplane/clear'),
}

// === OPA/Rego API ===
export const opaApi = {
  simulate: (data: any) => post<any>('/opa/simulate', data),
  defaultRules: () => get<any>('/opa/default-rules'),
  revision: () => get<any>('/opa/revision'),
  packBundle: (data: any) => post<any>('/opa/bundle/pack', data),
  hotLoad: (revision: string) => post<any>(`/opa/bundle/hot-load/${revision}`),
  sync: () => post<any>('/opa/bundle/sync'),
}
