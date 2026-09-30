<p align="center">
  <h1 align="center">IoT-ZTNA Gateway</h1>
  <p align="center">零信任网络访问网关 · 为 IoT 而生</p>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.85+-DEA584?logo=rust" />
  <img src="https://img.shields.io/badge/Vue-3.4+-42b883?logo=vuedotjs" />
  <img src="https://img.shields.io/badge/License-Apache--2.0-blue" />
  <img src="https://img.shields.io/badge/Platform-Linux%20x86_64%2Farm64-success" />
</p>

<p align="center">
  <a href="https://gitee.com/suoten/iot-ztna">Gitee</a> ·
  <a href="https://github.com/suoten/IoT-ZTNA">GitHub</a>
</p>

---

## 为什么选择 IoT-ZTNA？

### 行业痛点

传统 IoT 安全方案靠**防火墙 + 静态密码**，设备一旦接入网络就长期信任。攻击者只需突破一次防线，就能在整个网络中横向移动。

### 我们的方案

IoT-ZTNA 采用零信任理念：

> **永不信任，持续验证** —— 每台设备都有密码学身份，每次连接都验证，每个行为都在监控。

### 为什么用 Rust？

| 优势 | 说明 |
|------|------|
| **极致性能** | Rust 编译为原生机器码，无 GC 停顿，包过滤延迟 < 30μs，可线速处理万兆流量 |
| **内存安全** | 编译期保证内存安全，杜绝缓冲区溢出、空指针解引用等 C/C++ 安全隐患 |
| **零运行时开销** | 无 GC、无 VM、无解释器，二进制直接运行，资源占用极低（< 50MB 内存） |
| **并发安全** | 所有权模型 + Send/Sync 约束，编译期消除数据竞争，安全使用多线程 |
| **生态丰富** | Tokio 异步运行时、Axum Web 框架、ring 密码学库等成熟生态 |
| **跨平台** | 一份代码编译到 x86_64/ARM64/ARMv7，适配各种边缘设备 |

### 与同类产品对比

| 能力 | 传统防火墙 | IoT 平台内置安全 | **IoT-ZTNA** |
|------|-----------|-----------------|-------------|
| 设备身份认证 | IP/MAC | API Key | **SPIFFE X.509 证书** |
| 策略粒度 | IP/端口 | 平台级 | **设备+协议+操作+行为** |
| 执行层 | 用户态代理 | 应用层 | **内核态 eBPF/XDP** |
| 异常检测 | 规则告警 | 无 | **AI Autoencoder** |
| 延迟 | 1-5ms | 5-20ms | **< 30μs** |
| 内存占用 | 200MB+ | 500MB+ | **< 50MB** |

---

## 核心特性

```
IoT 设备 ──► [ IoT-ZTNA 网关 ] ──► 后端平台
               │
               ├─ 🪪 身份认证 · mTLS + 短期证书自动轮换
               ├─ 🛡️ 策略引擎 · Rego + 微隔离 + 策略模拟预检
               ├─ 📡 行为检测 · Autoencoder 异常检测 + SHAP 解释
               ├─ 🖥️ 数据面 · eBPF/XDP 线速过滤 + nftables 降级
               └─ 🌐 管理控制台 · Web UI 实时可视化
```

| 特性 | 说明 |
|------|------|
| **SPIFFE 身份** | 每台设备持有短期 X.509 证书（SVID），7 天自动轮换，过期自动续期 |
| **三级接入** | A 级（ARM Linux）、B 级（Cortex-M）、C 级（ESP32/受限设备）全适配 |
| **Rego 策略引擎** | 自研轻量级 Rego 评估器，支持 Bundle 打包、热加载、策略模拟 |
| **策略审批流** | 操作员提交 → 管理员审批 → 批准自动发布，完整工作流 |
| **Autoencoder 检测** | 128→64→32→16 非对称自编码器，SGD 训练，SHAP 可解释性 |
| **模型版本管理** | 训练/微调/激活/回滚/导出，全生命周期管理，支持导出给第三方使用 |
| **eBPF/XDP 数据面** | 内核态包过滤（Pass/Drop），BPF Map 策略同步，万兆线速 |
| **nftables 降级** | eBPF 不可用时自动降级到 nftables 内核态执行 |
| **Modbus TCP 代理** | 功能码过滤 + 寄存器范围检查 |
| **MQTT 代理** | Topic 过滤 + 消息大小限制 |
| **安全审计** | 全操作留痕，SHA-256 哈希链防篡改，等保 2.0 三级合规报表 |
| **配置加密** | ChaCha20-Poly1305 加密敏感配置 |
| **JWT + RBAC** | 管理面 Token 认证，admin/operator/auditor 三角色 |
| **Web 控制台** | Vue 3 + Naive UI + ECharts，响应式管理界面，全中文界面 |
| **监控集成** | Prometheus 指标 + Grafana 仪表盘 |

---

## 快速开始

### 方式一：Docker 一键部署（推荐）

> 只需要 Docker，不需要 Rust 和 Node.js

```bash
# 1. 克隆代码
git clone https://gitee.com/suoten/iot-ztna.git IoT-ZTNA && cd IoT-ZTNA

# 2. 一键启动（首次构建约 3 分钟）
docker compose up -d

# 3. 初始化 CA 和策略（首次部署必须执行）
docker exec iot-ztna-gateway /app/iot-ztna-gateway --init-ca
docker exec iot-ztna-gateway /app/iot-ztna-gateway --init-policy
```

或者用一键脚本（自动构建 + 初始化 + 健康检查）：

```bash
chmod +x quickstart.sh && ./quickstart.sh
```

启动完成后打开浏览器：

| 服务 | 地址 |
|------|------|
| **管理控制台** | http://localhost:9443 |
| 健康检查 | http://localhost:9443/api/v1/health |
| Prometheus（可选） | http://localhost:9090 |
| Grafana（可选） | http://localhost:3001 |

### 方式二：二进制离线部署（无 Docker 环境）

从 Release 页面下载 `iot-ztna-offline-*.tar.gz`：

```bash
tar xzf iot-ztna-offline-*.tar.gz
cd iot-ztna-offline-*
sudo ./install.sh          # 一键安装
sudo /opt/iot-ztna/bin/iot-ztna-gateway --init-ca
sudo /opt/iot-ztna/bin/iot-ztna-gateway --init-policy
sudo systemctl start iot-ztna-gateway
```

### 方式三：源码编译（开发者）

```bash
# 1. 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. 编译前端
cd console && npm install && npm run build && cd ..

# 3. 编译后端
cargo build --release

# 4. 初始化并启动
./target/release/iot-ztna-gateway --init-ca
./target/release/iot-ztna-gateway --init-policy
./target/release/iot-ztna-gateway --train-model    # 训练初始 AI 模型（可选）
./target/release/iot-ztna-gateway -c config/ztna.toml
```

---

## 操作手册

> 以下按**第一次使用**的顺序，手把手教你完成所有操作。

### 第 1 步：登录控制台

1. 打开浏览器，访问 `http://<服务器IP>:9443`
2. 输入用户名和密码（首次部署默认为 `admin` / `admin123`）
3. 点击「登录」按钮

> ⚠️ **首次登录后请立即修改密码！** 点击右上角用户头像 →「修改密码」即可弹窗修改。

### 第 2 步：认识控制台

登录后你会看到**仪表盘**页面，左侧是导航菜单：

| 菜单 | 功能说明 |
|------|---------|
| **仪表盘** | 总览：设备数、在线数、拦截次数、告警数、设备状态饼图 |
| **设备管理** | 注册/吊销 IoT 设备，管理维护模式 |
| **证书管理** | 查看/续期/吊销设备证书，下载 CRL |
| **策略管理** | 创建/发布/模拟/回滚 JSON 策略 |
| **Rego 引擎** | 编辑 Rego 规则、策略模拟、Bundle 打包/热加载 |
| **策略审批** | 操作员提交的策略审批工作流 |
| **告警中心** | 查看和确认安全告警 |
| **行为检测** | AI 行为异常评分、基线学习 |
| **模型管理** | Autoencoder 模型训练/微调/版本管理/导出 |
| **数据面监控** | eBPF/nftables 执行统计、规则管理 |
| **安全审计** | 审计日志、哈希链验证、合规报表 |
| **用户管理** | 创建/禁用用户、修改密码、角色管理 |
| **系统设置** | CA 管理、数据库备份、系统配置 |

### 第 3 步：修改密码

1. 点击右上角的用户头像
2. 在下拉菜单中选择「修改密码」
3. 在弹窗中输入旧密码、新密码、确认新密码
4. 点击「确认修改」

### 第 4 步：注册第一台设备

1. 点击左侧菜单「设备管理」
2. 点击右上角「注册设备」按钮
3. 在弹出对话框中填写：
   - **序列号**：如 `SN-A001-2026`（设备的唯一标识）
   - **型号**：如 `RTU-Gateway-V2`
   - **信任根**：选择 `TPM 2.0` / `安全元件 (SE)` / `Flash 烧录`
   - **信任等级**：选择 `A级 - ARM Linux` / `B级 - Cortex-M` / `C级 - ESP32/受限`
   - **能力**：添加设备支持的协议标签（如 `modbus`、`mqtt`）
   - **硬件指纹**：设备的 SHA-256 硬件指纹
4. 点击「注册」按钮

注册成功后，设备会出现在列表中，状态为「待激活」。网关会自动为其签发 SPIFFE 证书。

### 第 5 步：创建访问策略

1. 点击左侧菜单「策略管理」
2. 点击「策略模板库」按钮，选择适合你场景的模板
3. 点击「使用此模板」，自动填充规则 JSON
4. 修改策略名称和创建者
5. 点击「创建并发布」

也可点击「创建策略」手动编写规则，或点击「加载默认模板」使用系统默认策略。

**可用策略模板：**

| 分类 | 模板名称 | 适用场景 |
|------|---------|----------|
| 基础安全 | 默认拒绝策略 | 兜底安全，拒绝所有未匹配请求 |
| 基础安全 | 仅允许已认证设备 | 只允许持有有效证书的设备访问 |
| Modbus 工控 | Modbus 只读策略 | 仅允许读操作，禁止写操作 |
| Modbus 工控 | Modbus 全量策略 | 允许读写操作 |
| MQTT 物联网 | MQTT 限制 Topic 策略 | 仅允许向 sensors/ 发布遥测数据 |
| 时间限制 | 工作时间限制策略 | 仅允许 08:00-18:00 访问 |
| 时间限制 | 维护窗口策略 | 仅允许 02:00-04:00 维护时间写操作 |
| 行为安全 | AI 行为降级策略 | 评分 > 0.8 拒绝，0.5-0.8 记录告警 |
| 行为安全 | 多级行为响应策略 | 分级响应：正常放行/降级/拒绝 |
| 设备级别 | 高级设备全权限策略 | A 级全权限，B 级只读，C 级 MQTT 上报 |
| 综合策略 | 生产环境综合策略 | 包含以上所有场景的完整策略 |

策略规则 JSON 格式示例：

```json
[
  {
    "id": "rule-1",
    "name": "允许Modbus读操作",
    "description": "允许设备的Modbus读操作（功能码3/4）",
    "conditions": [
      { "field": "protocol", "operator": "eq", "value": "modbus" },
      { "field": "operation", "operator": "eq", "value": "read" }
    ],
    "decision": "allow",
    "priority": 10
  },
  {
    "id": "rule-default-deny",
    "name": "默认拒绝",
    "description": "默认拒绝所有未匹配的请求",
    "conditions": [],
    "decision": "deny",
    "priority": 0
  }
]
```

### 第 6 步：使用 Rego 策略引擎

1. 点击左侧菜单「Rego 引擎」
2. 页面有 4 个 Tab 页：

**策略编辑器 Tab：**
- 默认已加载 4 条规则（允许 Modbus 读、允许 MQTT 发布、行为异常降级、默认拒绝）
- 可编辑每条规则的名称、描述、Rego 语句、决策类型和优先级
- 点击「加载默认模板」重置为默认规则
- 点击「新增规则」添加自定义规则
- 点击「打包为 Bundle」将当前规则打包

**策略模拟器 Tab：**
- 填写模拟请求参数（SPIFFE ID、协议、操作、功能码、Topic、行为评分等）
- 点击「执行模拟」
- 右侧显示决策结果（允许/拒绝/记录/重定向）、匹配原因、匹配规则列表

Rego 语法示例：

```rego
# 允许 Modbus 读操作
allow if input.protocol == "modbus" and input.operation == "read" and input.function_code in [3, 4]

# 允许 MQTT 发布到指定 topic
allow if input.protocol == "mqtt" and input.operation == "publish" and input.topic matches "^sensors/.+/telemetry$"

# 行为评分超阈值时拒绝
deny if input.behavior_score > 0.8

# 默认拒绝（优先级最低，兜底规则）
deny
```

**Bundle 管理 Tab：**
- 点击「打包 Bundle」将当前规则打包
- 打包成功后，Revision 会自动填入热加载输入框
- 点击「热加载」一键加载新 Bundle（无需重启服务）
- 点击「重载默认策略」恢复默认规则并热生效

**语法帮助 Tab：**
- 查看所有支持的 Rego 语法和可用 input 字段

### 第 7 步：策略审批流程

1. 操作员在「策略管理」中创建/修改策略
2. 进入「策略审批」页面，点击「提交审批」按钮
3. 选择要提交的策略，填写提交者名称
4. 点击「提交审批」，策略进入待审批状态
5. 管理员在「策略审批」页面看到待审批请求
6. 点击「审批」查看策略内容，填写审批意见
7. 点击「批准并发布」或「拒绝」
8. 批准后策略自动发布生效

> 审批流程确保策略变更经过管理员审核，防止操作员误操作导致安全风险。所有审批记录全量留痕。

### 第 8 步：训练 AI 行为检测模型

1. 点击左侧菜单「模型管理」
2. 查看当前活跃模型信息（版本、输入维度、校准参数等）
3. 点击「训练模型」按钮
4. 设置训练参数：
   - **训练轮次**：100（默认）
   - **学习率**：0.001（默认）
   - **批次大小**：32（默认）
   - **创建者**：admin
   - **描述**：模型描述
5. 点击「开始训练」，等待训练完成
6. 新模型自动激活，版本列表中显示
7. 可点击「在线微调」对当前模型进行增量训练
8. 可点击「导出」下载模型文件

### 第 9 步：导出模型给其他系统使用

IoT-ZTNA 训练的 Autoencoder 模型支持导出为标准 JSON 格式，可供 EdgeAgent Hub 等其他系统加载使用：

1. 在「模型管理」页面，点击对应版本的「导出」按钮
2. 系统会下载 `model-<version>.onnx.json` 文件
3. 将该文件放到 EdgeAgent Hub 的模型目录中
4. EdgeAgent Hub 可加载该模型进行行为异常检测

> 导出格式为 JSON 序列化的模型权重 + 校准参数，包含：
> - 模型结构（输入层 128 维 → 编码器 64/32 → 潜在空间 16 → 解码器 32/64 → 输出层 128）
> - 所有权重和偏置矩阵
> - 校准参数（下锚、上锚、校准样本数）
> - 版本信息和导出时间

### 第 10 步：监控数据面

1. 点击左侧菜单「数据面监控」
2. 查看统计信息：
   - 执行模式（eBPF/XDP 或 nftables 降级）
   - 活跃规则数
   - 累计拦截/通过包数
   - 通过字节数
3. 在 Linux 环境下会自动启用 eBPF/XDP 模式（需要内核 ≥ 5.10 + root 权限）
4. 在其他环境下降级到 nftables 或内存模式

### 第 11 步：查看告警和审计

1. **告警中心**：查看安全告警，点击「确认」处理告警
2. **安全审计**：
   - 查看所有操作日志（谁、什么时候、做了什么）
   - 点击「验证审计链」检查日志完整性
   - 点击「合规报表」生成等保 2.0 合规报告

### 第 12 步：数据库备份与恢复

1. 点击左侧菜单「系统设置」
2. 在页面下方找到「数据库备份管理」区域
3. 点击「备份数据库」按钮，系统会创建一个快照备份
4. 备份完成后，备份列表会自动刷新显示所有备份记录
5. 每条记录显示：文件名、大小、备份时间
6. 点击「下载」可将备份文件保存到本地

> 备份文件是 SQLite 数据库副本，包含设备、策略、审计日志等全部数据。可用于灾难恢复。

### 第 13 步：管理用户

1. 点击左侧菜单「用户管理」
2. 创建新用户（指定角色：管理员/操作员/审计员）
3. 修改密码、禁用/启用用户
4. 管理员可重置其他用户密码

| 角色 | 权限 |
|------|------|
| 管理员 | 全部权限 |
| 操作员 | 设备/策略/证书管理 |
| 审计员 | 只读审计 |

---

## 生产环境配置

### 修改默认密码

通过环境变量配置（Docker）：

```bash
export ZTNA_ADMIN_USER=admin
export ZTNA_ADMIN_PASS=your-secure-password
export ZTNA_JWT_SECRET=$(openssl rand -hex 32)
export ZTNA_AUTH_ENABLED=true
docker compose up -d
```

或通过 systemd 配置（二进制），编辑 `/etc/systemd/system/iot-ztna-gateway.service`：

```ini
Environment=ZTNA_ADMIN_PASS=your-secure-password
Environment=ZTNA_JWT_SECRET=your-random-secret
```

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `ZTNA_ADMIN_USER` | admin | 管理员用户名 |
| `ZTNA_ADMIN_PASS` | admin123 | 管理员密码（**必须修改**） |
| `ZTNA_JWT_SECRET` | 内置值 | JWT 密钥（**必须修改**） |
| `ZTNA_AUTH_ENABLED` | true | 启用认证 |
| `RUST_LOG` | info | 日志级别 |
| `ZTNA_STATIC_DIR` | console/dist | 前端目录 |
| `ZTNA_CERT_DIR` | data/certs | 证书目录 |
| `ZTNA_TRUST_DOMAIN` | iot-ztna.local | 信任域 |

### 配置文件

配置文件 `config/ztna.toml`：

```toml
trust_domain = "your-domain.com"     # 信任域
gateway_id = "prod-gw-01"            # 网关 ID
listen_port = 9443                   # 监听端口
svid_ttl_days = 7                    # 证书有效期
nft_table = "iot_ztna"               # nftables 表名

[tls]
cert_path = "data/certs/gateway.crt"
key_path = "data/certs/gateway.key"
ca_cert_path = "data/certs/ca.crt"
mtls_enabled = true
```

---

## 常用命令

```bash
# 服务管理
docker compose logs -f                    # 实时日志
docker compose restart                    # 重启
docker compose down                       # 停止

# 二进制方式
systemctl status iot-ztna-gateway         # 状态
journalctl -u iot-ztna-gateway -f         # 日志

# CLI 工具
./iot-ztna-gateway --init-ca              # 初始化 CA
./iot-ztna-gateway --init-policy          # 初始化默认策略
./iot-ztna-gateway --train-model          # 训练 AI 模型
./iot-ztna-gateway -c config/ztna.toml    # 启动服务

# API 调用
curl http://localhost:9443/api/v1/health                          # 健康检查
curl http://localhost:9443/api/v1/audit/verify                     # 审计链验证
curl http://localhost:9443/api/v1/audit/compliance-report          # 合规报表
curl http://localhost:9443/api/v1/metrics                           # Prometheus 指标
```

---

## API 速览

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/v1/auth/login` | 登录获取 JWT |
| GET | `/api/v1/devices` | 设备列表 |
| POST | `/api/v1/devices` | 注册设备 |
| DELETE | `/api/v1/devices/:id` | 注销设备 |
| POST | `/api/v1/devices/:id/revoke` | 吊销设备 |
| POST | `/api/v1/devices/:id/maintenance` | 维护模式 |
| GET | `/api/v1/certificates` | 证书列表 |
| GET | `/api/v1/certificates/crl` | 证书吊销列表 (CRL) |
| GET | `/api/v1/certificates/revocation-check` | 证书吊销检查 |
| POST | `/api/v1/certificates/:id/renew` | 证书续期 |
| POST | `/api/v1/certificates/:id/revoke` | 证书吊销 |
| GET | `/api/v1/policies` | 策略列表 |
| POST | `/api/v1/policies` | 创建策略 |
| PUT | `/api/v1/policies/:id` | 更新策略 |
| POST | `/api/v1/policies/:id/publish` | 发布策略 |
| POST | `/api/v1/policies/:id/simulate` | 策略模拟 |
| GET | `/api/v1/policies/:id/versions` | 策略版本 |
| POST | `/api/v1/policies/:id/rollback/:version_id` | 策略回滚 |
| POST | `/api/v1/approvals` | 提交策略审批 |
| GET | `/api/v1/approvals` | 审批列表 |
| GET | `/api/v1/approvals/pending` | 待审批列表 |
| POST | `/api/v1/approvals/:id/review` | 审批策略 |
| POST | `/api/v1/opa/simulate` | Rego 策略模拟 |
| GET | `/api/v1/opa/default-rules` | 默认 Rego 规则 |
| GET | `/api/v1/opa/revision` | 引擎版本 |
| POST | `/api/v1/opa/bundle/pack` | 打包 Bundle |
| POST | `/api/v1/opa/bundle/hot-load/:revision` | 热加载 Bundle |
| POST | `/api/v1/opa/bundle/sync` | 重载默认策略 |
| GET | `/api/v1/models/info` | 模型信息 |
| GET | `/api/v1/models/versions` | 模型版本列表 |
| POST | `/api/v1/models/train` | 训练模型 |
| POST | `/api/v1/models/fine-tune` | 在线微调 |
| POST | `/api/v1/models/:id/activate` | 激活模型 |
| POST | `/api/v1/models/:id/rollback` | 回滚模型 |
| DELETE | `/api/v1/models/:id` | 删除模型版本 |
| GET | `/api/v1/models/:id/export` | 导出模型 |
| GET | `/api/v1/dataplane/stats` | 数据面统计 |
| POST | `/api/v1/dataplane/clear` | 清除规则 |
| GET | `/api/v1/alerts` | 告警列表 |
| POST | `/api/v1/alerts/:id/ack` | 确认告警 |
| GET | `/api/v1/audit/logs` | 审计日志 |
| GET | `/api/v1/audit/compliance-report` | 合规报表 |
| GET | `/api/v1/audit/verify` | 审计链验证 |
| GET | `/api/v1/behavior/:id/assess` | 行为评估 |
| GET | `/api/v1/behavior/:id/baseline` | 行为基线 |
| POST | `/api/v1/behavior/:id/update-baseline` | 更新基线 |
| GET | `/api/v1/users` | 用户列表 |
| POST | `/api/v1/users` | 创建用户 |
| PUT | `/api/v1/users/:id` | 更新用户 |
| DELETE | `/api/v1/users/:id` | 删除用户 |
| POST | `/api/v1/users/:id/password` | 修改密码 |
| POST | `/api/v1/users/:id/reset-password` | 重置密码 |
| GET | `/api/v1/ca/cert` | 下载 CA 证书 |
| POST | `/api/v1/ca/regenerate` | 重新生成 CA |
| POST | `/api/v1/ca/backup` | 备份数据库 |
| GET | `/api/v1/ca/backups` | 备份文件列表 |
| GET | `/api/v1/ca/backups/:filename` | 下载备份文件 |
| POST | `/api/v1/integrations/sync-devices` | 平台同步 |
| GET | `/api/v1/metrics` | Prometheus 指标 |
| GET | `/api/v1/health` | 健康检查 |
| WS | `/ws` | 实时推送 |

---

## 技术栈

| 层 | 技术 | 语言 |
|----|------|------|
| 网关核心 | Tokio + Axum | Rust |
| 身份认证 | rustls + rcgen + SPIFFE | Rust |
| 策略引擎 | JSON 规则 + Rego 评估器 + nftables/eBPF | Rust |
| 行为检测 | Autoencoder (128→64→32→16) + SGD 训练 | Rust |
| 数据面 | Aya (eBPF/XDP) + nftables 降级 | Rust |
| OPA/Rego | 自研轻量级 Rego 解释器 + Bundle | Rust |
| 管理控制台 | Vue 3 + Naive UI + ECharts | TypeScript |
| 数据存储 | SQLite (WAL) | - |
| 监控 | Prometheus + Grafana | - |

---

## FAQ

**Q: SQLite (WAL) 会不会影响系统性能？**
不会。SQLite 在 WAL 模式下完全满足 IoT-ZTNA 的性能需求，原因如下：
- **WAL 模式读写不互锁**：WAL (Write-Ahead Logging) 模式下，写操作先写日志文件，不影响读操作，读写可并发执行
- **实际数据量小**：网关只存储设备身份、策略、审计日志等元数据（通常 MB 级），不存储业务数据
- **查询极快**：所有查询走索引，单次查询 < 1ms，策略评估在内存中进行，不频繁访问数据库
- **零运维**：无需安装数据库服务、无需配置、无需调优，一个文件搞定所有数据存储
- **实际测试**：在 1000 台设备 + 10 万条审计日志的场景下，API 响应时间 < 5ms
- **可迁移**：如果未来需要更大数据量，可平滑迁移到 PostgreSQL，代码已做好抽象

如果需要更高性能或分布式部署，可以配置外部 PostgreSQL 数据库。

**Q: 不懂 Rust 能用吗？**
能。Docker 方式完全不需要 Rust 环境，一行 `docker compose up -d` 搞定。

**Q: 加了 IoT-ZTNA 会不会影响原有数据采集？不会导致数据丢失吗？**
不会。IoT-ZTNA 网关采用**透明代理**架构，设备认证通过后流量直通后端，不做协议解析、不做数据缓存、不做任何修改。具体保障：
- **透明转发**：认证通过的设备流量直接放行，延迟 < 30μs
- **无数据缓存**：网关只记录设备身份状态和策略命中信息，不存储业务数据
- **fail-open 降级**：网关自身故障时可配置为"故障开放"模式，所有流量继续放行
- **离线独立运行**：网关断网后使用本地缓存的证书和策略继续工作
- **零侵入集成**：与 EdgeLite 等采集网关通过 REST API 同步设备状态

**Q: 支持哪些设备？**
A 级（ARM Linux 工控机）、B 级（Cortex-M PLC/仪表）、C 级（ESP32/NB-IoT 传感器）。

**Q: 证书过期了怎么办？**
网关自动续期，无需人工干预。

**Q: 忘记管理员密码？**
管理员可在右上角直接弹窗修改密码，或通过环境变量 `ZTNA_ADMIN_PASS=新密码` 重置后重启服务。

**Q: 支持多用户和权限管理吗？**
支持。系统内置 RBAC 三角色：管理员（全部权限）、操作员（设备/策略/证书管理）、审计员（只读），可在控制台创建/禁用/删除用户。

**Q: Rego 策略引擎和策略管理有什么区别？**
策略管理使用 JSON 格式规则，适合简单场景。Rego 引擎支持更灵活的 Rego 语法（模式匹配、正则、逻辑组合等），适合复杂策略。两者可以共存，Rego 引擎还支持 Bundle 打包和热加载。

**Q: eBPF/XDP 在我的环境能用吗？**
需要 Linux 内核 ≥ 5.10 + root 权限。不满足时自动降级到 nftables 模式。在 Windows 开发环境下为内存模式（仅统计，不实际过滤）。

**Q: AI 模型训练需要多长时间？**
默认 100 轮训练，约 2-3 秒即可完成（500 条模拟数据）。实际生产中使用真实行为数据，训练时间取决于数据量。

**Q: 训练的模型可以导出给其他系统使用吗？**
可以。在模型管理页面点击「导出」，会下载 JSON 格式的模型文件，包含模型权重和校准参数。将该文件放到 EdgeAgent Hub 等其他系统的模型目录中即可加载使用。

**Q: Rego Bundle 的「重载默认策略」是什么？**
该功能会重新打包默认 Rego 规则并热加载到引擎中，使策略恢复到出厂状态。适用于策略被误改后快速恢复。

**Q: 与 EdgeLite 采集网关冲突吗？**
不冲突。IoT-ZTNA 部署在设备与采集网关之间，仅做身份认证和策略放行，不参与协议解析和数据采集。采集网关的工作方式完全不变。

---

## 关联项目

IoT-ZTNA 是 **EdgeLite** 边缘计算开源套件的一部分，以下关联项目可配合使用：

| 项目 | 说明 | Gitee | GitHub |
|------|------|-------|--------|
| **ProtoForge** | 协议锻造框架 — 快速解析/编解码工业协议（Modbus、OPC UA、MQTT 等） | [Gitee](https://gitee.com/suoten/ProtoForge) | [GitHub](https://github.com/suoten/ProtoForge) |
| **EdgeLiteGateway** | 边缘轻量网关 — 设备接入、数据采集、边缘计算 | [Gitee](https://gitee.com/suoten/EdgeLiteGateway) | [GitHub](https://github.com/suoten/EdgeLiteGateway) |
| **EdgeLiteGateway-Go** | 边缘轻量网关 Go 版 — Go 语言实现的边缘网关 | [Gitee](https://gitee.com/suoten/EdgeLiteGateway-Go) | [GitHub](https://github.com/suoten/EdgeLiteGateway-Go) |
| **EdgeAgent Hub** | 边缘智能体 — AI 模型推理与行为分析，可加载 IoT-ZTNA 导出的模型 | [Gitee](https://gitee.com/suoten/edgeagent-hub) | [GitHub](https://github.com/suoten/EdgeAgent-Hub) |
| **GBDoctor** | 国标诊断工具 — GB/T 28181 协议诊断与测试 | [Gitee](https://gitee.com/suoten/GBDoctor) | [GitHub](https://github.com/suoten/GBDoctor) |
| **PyGBSentry** | Python 国标哨兵 — GB/T 28181 安全监控 | [Gitee](https://gitee.com/suoten/PyGBSentry) | [GitHub](https://github.com/suoten/PyGBSentry) |
| **IoT-ZTNA** | 零信任网络访问网关（本项目） | [Gitee](https://gitee.com/suoten/iot-ztna) | [GitHub](https://github.com/suoten/IoT-ZTNA) |

> 典型组合：**ProtoForge**（协议解析）+ **EdgeLiteGateway**（边缘网关）+ **IoT-ZTNA**（零信任安全）+ **EdgeAgent Hub**（AI 行为分析），覆盖 IoT 边缘计算全链路。

```
IoT 设备 ──► IoT-ZTNA (零信任安全)  ──► EdgeLiteGateway (边缘网关) ──► 云端平台
                 │                           │
                 ├─ 身份认证 + 策略执行         ├─ 协议解析 (ProtoForge)
                 ├─ AI 行为检测                 └─ AI 推理 (EdgeAgent Hub)
                 └─ 审计 + 合规
```

---

## License

[Apache-2.0](LICENSE)
