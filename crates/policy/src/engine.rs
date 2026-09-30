//! 策略决策引擎
//!
//! 轻量级 Rego-like 策略评估，解析 JSON 规则定义并评估决策。
//! 支持: 设备级、协议级、操作级、时间窗口、行为降级等微隔离策略。

use chrono::{Timelike, Utc};
use iot_ztna_common::{
    Error, PolicyDecision, PolicyRequest, PolicyResponse, PolicyStatus, Result,
};
use iot_ztna_storage::Database;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// 策略规则定义（JSON 格式，易于编辑和版本管理）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PolicyRule {
    /// 规则 ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 规则描述
    pub description: String,
    /// 匹配条件
    pub conditions: Vec<Condition>,
    /// 决策结果
    pub decision: PolicyDecision,
    /// 优先级（数字越大优先级越高）
    pub priority: i32,
}

/// 策略匹配条件
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Condition {
    /// 字段名
    pub field: String,
    /// 操作符
    pub operator: Operator,
    /// 期望值
    pub value: serde_json::Value,
}

/// 条件操作符
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operator {
    /// 等于
    Eq,
    /// 不等于
    Ne,
    /// 在列表中
    In,
    /// 不在列表中
    NotIn,
    /// 小于
    Lt,
    /// 大于
    Gt,
    /// 小于等于
    Le,
    /// 大于等于
    Ge,
    /// 正则匹配
    Regex,
    /// 时间窗口（hour 范围）
    TimeWindow,
}

/// 策略引擎
pub struct PolicyEngine {
    db: Database,
    rules: Arc<RwLock<Vec<PolicyRule>>>,
}

impl PolicyEngine {
    pub fn new(db: Database) -> Self {
        let engine = Self {
            db,
            rules: Arc::new(RwLock::new(Vec::new())),
        };
        engine
    }

    /// 从数据库加载已发布策略
    pub fn load_policies(&self) -> Result<()> {
        let policies = self.db.get_published_policies()?;
        let mut all_rules = Vec::new();

        for policy in policies {
            match serde_json::from_str::<Vec<PolicyRule>>(&policy.rego_content) {
                Ok(rules) => all_rules.extend(rules),
                Err(e) => {
                    tracing::warn!(
                        policy_id = %policy.id,
                        error = %e,
                        "策略解析失败，跳过"
                    );
                }
            }
        }

        // 按优先级降序排列
        all_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        let rule_count = all_rules.len();
        let mut rules_lock = self.rules.write().map_err(|e| {
            Error::Internal(format!("获取规则写锁失败: {}", e))
        })?;
        *rules_lock = all_rules;

        tracing::info!(rule_count = rule_count, "策略加载完成");
        Ok(())
    }

    /// 评估策略决策
    pub fn evaluate(&self, request: &PolicyRequest) -> Result<PolicyResponse> {
        let rules = self.rules.read().map_err(|e| {
            Error::Internal(format!("获取规则读锁失败: {}", e))
        })?;

        let mut matched_rules = Vec::new();
        let now = Utc::now();

        // 默认拒绝
        let mut final_decision = PolicyDecision::Deny;
        let mut final_reason = "默认拒绝：无匹配规则".to_string();

        for rule in rules.iter() {
            if self.match_conditions(&rule.conditions, request, &now) {
                matched_rules.push(rule.name.clone());
                tracing::debug!(
                    rule = %rule.name,
                    decision = %rule.decision,
                    "规则匹配"
                );
                // 第一个匹配的高优先级规则决定结果
                if final_decision == PolicyDecision::Deny {
                    final_decision = rule.decision.clone();
                    final_reason = format!("匹配规则: {}", rule.name);
                }
            }
        }

        // 记录决策审计
        self.db.audit(
            "policy_engine",
            "evaluate",
            &request.device_spiffe_id.to_string(),
            &final_decision.to_string(),
            serde_json::json!({
                "protocol": request.protocol,
                "operation": request.operation,
                "target": request.target,
                "matched_rules": matched_rules,
            }),
            "",
        )?;

        Ok(PolicyResponse {
            decision: final_decision,
            reason: final_reason,
            matched_rules,
            timestamp: now,
        })
    }

    /// 匹配条件列表
    fn match_conditions(
        &self,
        conditions: &[Condition],
        request: &PolicyRequest,
        now: &chrono::DateTime<Utc>,
    ) -> bool {
        conditions.iter().all(|cond| {
            self.match_condition(cond, request, now)
        })
    }

    /// 匹配单个条件
    fn match_condition(
        &self,
        cond: &Condition,
        request: &PolicyRequest,
        now: &chrono::DateTime<Utc>,
    ) -> bool {
        match cond.field.as_str() {
            "spiffe_id" => self.compare_str(
                &request.device_spiffe_id.to_string(),
                &cond.operator,
                &cond.value,
            ),
            "protocol" => {
                self.compare_str(&request.protocol, &cond.operator, &cond.value)
            }
            "operation" => {
                self.compare_str(&request.operation, &cond.operator, &cond.value)
            }
            "target" => {
                self.compare_str(&request.target, &cond.operator, &cond.value)
            }
            "function_code" => {
                if let Some(fc) = request.function_code {
                    self.compare_num(fc as f64, &cond.operator, &cond.value)
                } else {
                    false
                }
            }
            "topic" => {
                if let Some(topic) = &request.topic {
                    self.compare_str(topic, &cond.operator, &cond.value)
                } else {
                    false
                }
            }
            "behavior_score" => {
                if let Some(score) = request.behavior_score {
                    self.compare_num(score, &cond.operator, &cond.value)
                } else {
                    false
                }
            }
            "maintenance_mode" => request.maintenance_mode,
            "time_hour" => match &cond.operator {
                Operator::TimeWindow => {
                    if let serde_json::Value::Array(arr) = &cond.value {
                        if arr.len() >= 2 {
                            let start: u32 =
                                arr[0].as_u64().unwrap_or(0) as u32;
                            let end: u32 =
                                arr[1].as_u64().unwrap_or(24) as u32;
                            let hour = now.hour();
                            if start <= end {
                                hour >= start && hour < end
                            } else {
                                // 跨午夜
                                hour >= start || hour < end
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn compare_str(&self, actual: &str, op: &Operator, expected: &serde_json::Value) -> bool {
        match op {
            Operator::Eq => expected.as_str().map(|s| actual == s).unwrap_or(false),
            Operator::Ne => expected.as_str().map(|s| actual != s).unwrap_or(true),
            Operator::In => {
                if let Some(arr) = expected.as_array() {
                    arr.iter().any(|v| v.as_str() == Some(actual))
                } else {
                    false
                }
            }
            Operator::NotIn => {
                if let Some(arr) = expected.as_array() {
                    !arr.iter().any(|v| v.as_str() == Some(actual))
                } else {
                    true
                }
            }
            Operator::Regex => {
                if let Some(pattern) = expected.as_str() {
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(actual))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn compare_num(&self, actual: f64, op: &Operator, expected: &serde_json::Value) -> bool {
        let exp = expected.as_f64().unwrap_or(f64::MAX);
        match op {
            Operator::Eq => (actual - exp).abs() < f64::EPSILON,
            Operator::Ne => (actual - exp).abs() > f64::EPSILON,
            Operator::Lt => actual < exp,
            Operator::Gt => actual > exp,
            Operator::Le => actual <= exp,
            Operator::Ge => actual >= exp,
            Operator::In => {
                if let Some(arr) = expected.as_array() {
                    arr.iter().any(|v| v.as_f64() == Some(actual))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// 创建新策略
    pub fn create_policy(
        &self,
        name: &str,
        rules: Vec<PolicyRule>,
        created_by: &str,
    ) -> Result<String> {
        let policy_id = Uuid::new_v4().to_string();
        let rego_content = serde_json::to_string_pretty(&rules)?;

        let policy = iot_ztna_common::Policy {
            id: policy_id.clone(),
            version: "1.0".to_string(),
            name: name.to_string(),
            rego_content,
            status: PolicyStatus::Draft,
            created_by: created_by.to_string(),
            created_at: chrono::Utc::now(),
            published_at: None,
        };
        self.db.insert_policy(&policy)?;

        tracing::info!(policy_id = %policy_id, name = name, "策略创建成功");
        Ok(policy_id)
    }

    /// 发布策略（热加载）
    pub fn publish_policy(&self, policy_id: &str) -> Result<()> {
        self.db.update_policy_status(policy_id, &PolicyStatus::Published)?;
        // 重新加载策略
        self.load_policies()?;
        tracing::info!(policy_id = policy_id, "策略已发布并热加载");
        Ok(())
    }

    /// 模拟策略（dry-run 模式，不执行只记录）
    pub fn simulate(&self, request: &PolicyRequest) -> Result<PolicyResponse> {
        let response = self.evaluate(request)?;
        tracing::info!(
            spiffe_id = %request.device_spiffe_id,
            decision = %response.decision,
            "策略模拟 (dry-run)"
        );
        // 记录模拟审计
        self.db.audit(
            "system",
            "simulate_policy",
            &request.device_spiffe_id.to_string(),
            &format!("dry-run:{}", response.decision),
            serde_json::json!({
                "reason": response.reason,
                "matched_rules": response.matched_rules,
            }),
            "",
        )?;
        Ok(response)
    }

    /// 默认策略规则模板
    pub fn default_rules() -> Vec<PolicyRule> {
        vec![
            PolicyRule {
                id: "default-deny".to_string(),
                name: "默认拒绝".to_string(),
                description: "默认拒绝所有请求".to_string(),
                conditions: vec![],
                decision: PolicyDecision::Deny,
                priority: 0,
            },
            PolicyRule {
                id: "modbus-read-allow".to_string(),
                name: "允许Modbus读操作".to_string(),
                description: "允许设备的Modbus读操作（功能码3/4）".to_string(),
                conditions: vec![
                    Condition {
                        field: "protocol".to_string(),
                        operator: Operator::Eq,
                        value: serde_json::json!("modbus"),
                    },
                    Condition {
                        field: "operation".to_string(),
                        operator: Operator::Eq,
                        value: serde_json::json!("read"),
                    },
                    Condition {
                        field: "function_code".to_string(),
                        operator: Operator::In,
                        value: serde_json::json!([3, 4]),
                    },
                ],
                decision: PolicyDecision::Allow,
                priority: 10,
            },
            PolicyRule {
                id: "mqtt-publish-allow".to_string(),
                name: "允许MQTT发布".to_string(),
                description: "允许设备向指定topic发布MQTT消息".to_string(),
                conditions: vec![
                    Condition {
                        field: "protocol".to_string(),
                        operator: Operator::Eq,
                        value: serde_json::json!("mqtt"),
                    },
                    Condition {
                        field: "operation".to_string(),
                        operator: Operator::Eq,
                        value: serde_json::json!("publish"),
                    },
                    Condition {
                        field: "topic".to_string(),
                        operator: Operator::Regex,
                        value: serde_json::json!("^sensors/.+/telemetry$"),
                    },
                ],
                decision: PolicyDecision::Allow,
                priority: 10,
            },
            PolicyRule {
                id: "behavior-degrade".to_string(),
                name: "行为异常降级".to_string(),
                description: "行为评分超过0.8时降级为只读".to_string(),
                conditions: vec![Condition {
                    field: "behavior_score".to_string(),
                    operator: Operator::Ge,
                    value: serde_json::json!(0.8),
                }],
                decision: PolicyDecision::Deny,
                priority: 50,
            },
            PolicyRule {
                id: "maintenance-write".to_string(),
                name: "维护窗口写操作".to_string(),
                description: "维护模式（02:00-04:00）内允许写操作".to_string(),
                conditions: vec![
                    Condition {
                        field: "time_hour".to_string(),
                        operator: Operator::TimeWindow,
                        value: serde_json::json!([2, 4]),
                    },
                    Condition {
                        field: "maintenance_mode".to_string(),
                        operator: Operator::Eq,
                        value: serde_json::json!(true),
                    },
                    Condition {
                        field: "operation".to_string(),
                        operator: Operator::Eq,
                        value: serde_json::json!("write"),
                    },
                ],
                decision: PolicyDecision::Allow,
                priority: 20,
            },
        ]
    }

    /// 策略模板库
    /// 
    /// 返回分类的常用策略模板，供前端选择加载
    pub fn policy_templates() -> Vec<PolicyTemplate> {
        vec![
            // === 基础安全 ===
            PolicyTemplate {
                category: "基础安全".to_string(),
                name: "默认拒绝策略".to_string(),
                description: "拒绝所有未匹配的请求，作为兜底安全策略".to_string(),
                rules: vec![
                    PolicyRule {
                        id: "default-deny".to_string(),
                        name: "默认拒绝".to_string(),
                        description: "默认拒绝所有请求".to_string(),
                        conditions: vec![],
                        decision: PolicyDecision::Deny,
                        priority: 0,
                    },
                ],
            },
            PolicyTemplate {
                category: "基础安全".to_string(),
                name: "仅允许已认证设备".to_string(),
                description: "只允许持有有效证书的已认证设备访问".to_string(),
                rules: vec![
                    PolicyRule {
                        id: "allow-authenticated".to_string(),
                        name: "允许已认证设备".to_string(),
                        description: "允许持有有效证书的设备访问".to_string(),
                        conditions: vec![
                            Condition {
                                field: "authenticated".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!(true),
                            },
                        ],
                        decision: PolicyDecision::Allow,
                        priority: 10,
                    },
                    PolicyRule {
                        id: "default-deny".to_string(),
                        name: "默认拒绝".to_string(),
                        description: "拒绝所有未匹配的请求".to_string(),
                        conditions: vec![],
                        decision: PolicyDecision::Deny,
                        priority: 0,
                    },
                ],
            },
            // === Modbus 工控 ===
            PolicyTemplate {
                category: "Modbus 工控".to_string(),
                name: "Modbus 只读策略".to_string(),
                description: "仅允许 Modbus 读操作（功能码 3/4），禁止写操作".to_string(),
                rules: vec![
                    PolicyRule {
                        id: "modbus-read-allow".to_string(),
                        name: "允许Modbus读".to_string(),
                        description: "允许 Modbus 读操作（功能码 3/4）".to_string(),
                        conditions: vec![
                            Condition {
                                field: "protocol".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("modbus"),
                            },
                            Condition {
                                field: "operation".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("read"),
                            },
                            Condition {
                                field: "function_code".to_string(),
                                operator: Operator::In,
                                value: serde_json::json!([3, 4]),
                            },
                        ],
                        decision: PolicyDecision::Allow,
                        priority: 10,
                    },
                    PolicyRule {
                        id: "modbus-write-deny".to_string(),
                        name: "禁止Modbus写".to_string(),
                        description: "禁止所有 Modbus 写操作（功能码 5/6/15/16）".to_string(),
                        conditions: vec![
                            Condition {
                                field: "protocol".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("modbus"),
                            },
                            Condition {
                                field: "function_code".to_string(),
                                operator: Operator::In,
                                value: serde_json::json!([5, 6, 15, 16]),
                            },
                        ],
                        decision: PolicyDecision::Deny,
                        priority: 15,
                    },
                    PolicyRule {
                        id: "default-deny".to_string(),
                        name: "默认拒绝".to_string(),
                        description: "默认拒绝所有请求".to_string(),
                        conditions: vec![],
                        decision: PolicyDecision::Deny,
                        priority: 0,
                    },
                ],
            },
            PolicyTemplate {
                category: "Modbus 工控".to_string(),
                name: "Modbus 全量策略".to_string(),
                description: "允许 Modbus 读写操作，但限制寄存器范围".to_string(),
                rules: vec![
                    PolicyRule {
                        id: "modbus-read-allow".to_string(),
                        name: "允许Modbus读".to_string(),
                        description: "允许 Modbus 读操作".to_string(),
                        conditions: vec![
                            Condition {
                                field: "protocol".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("modbus"),
                            },
                            Condition {
                                field: "operation".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("read"),
                            },
                        ],
                        decision: PolicyDecision::Allow,
                        priority: 10,
                    },
                    PolicyRule {
                        id: "modbus-write-allow".to_string(),
                        name: "允许Modbus写".to_string(),
                        description: "允许 Modbus 写操作（功能码 5/6/15/16）".to_string(),
                        conditions: vec![
                            Condition {
                                field: "protocol".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("modbus"),
                            },
                            Condition {
                                field: "operation".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("write"),
                            },
                            Condition {
                                field: "function_code".to_string(),
                                operator: Operator::In,
                                value: serde_json::json!([5, 6, 15, 16]),
                            },
                        ],
                        decision: PolicyDecision::Allow,
                        priority: 10,
                    },
                    PolicyRule {
                        id: "default-deny".to_string(),
                        name: "默认拒绝".to_string(),
                        description: "默认拒绝所有请求".to_string(),
                        conditions: vec![],
                        decision: PolicyDecision::Deny,
                        priority: 0,
                    },
                ],
            },
            // === MQTT 物联网 ===
            PolicyTemplate {
                category: "MQTT 物联网".to_string(),
                name: "MQTT 限制 Topic 策略".to_string(),
                description: "仅允许设备向 sensors/ 下发布遥测数据".to_string(),
                rules: vec![
                    PolicyRule {
                        id: "mqtt-publish-allow".to_string(),
                        name: "允许MQTT发布".to_string(),
                        description: "允许设备向 sensors/+/telemetry topic 发布消息".to_string(),
                        conditions: vec![
                            Condition {
                                field: "protocol".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("mqtt"),
                            },
                            Condition {
                                field: "operation".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("publish"),
                            },
                            Condition {
                                field: "topic".to_string(),
                                operator: Operator::Regex,
                                value: serde_json::json!("^sensors/.+/telemetry$"),
                            },
                        ],
                        decision: PolicyDecision::Allow,
                        priority: 10,
                    },
                    PolicyRule {
                        id: "mqtt-subscribe-allow".to_string(),
                        name: "允许MQTT订阅".to_string(),
                        description: "允许设备订阅 commands/+/ctrl 下发指令 topic".to_string(),
                        conditions: vec![
                            Condition {
                                field: "protocol".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("mqtt"),
                            },
                            Condition {
                                field: "operation".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("subscribe"),
                            },
                            Condition {
                                field: "topic".to_string(),
                                operator: Operator::Regex,
                                value: serde_json::json!("^commands/.+/ctrl$"),
                            },
                        ],
                        decision: PolicyDecision::Allow,
                        priority: 10,
                    },
                    PolicyRule {
                        id: "default-deny".to_string(),
                        name: "默认拒绝".to_string(),
                        description: "默认拒绝所有请求".to_string(),
                        conditions: vec![],
                        decision: PolicyDecision::Deny,
                        priority: 0,
                    },
                ],
            },
            // === 时间限制 ===
            PolicyTemplate {
                category: "时间限制".to_string(),
                name: "工作时间限制策略".to_string(),
                description: "仅允许工作时间（08:00-18:00）内的访问".to_string(),
                rules: vec![
                    PolicyRule {
                        id: "worktime-allow".to_string(),
                        name: "工作时间允许".to_string(),
                        description: "工作时间（08:00-18:00）内允许访问".to_string(),
                        conditions: vec![
                            Condition {
                                field: "time_hour".to_string(),
                                operator: Operator::TimeWindow,
                                value: serde_json::json!([8, 18]),
                            },
                        ],
                        decision: PolicyDecision::Allow,
                        priority: 10,
                    },
                    PolicyRule {
                        id: "default-deny".to_string(),
                        name: "默认拒绝".to_string(),
                        description: "非工作时间拒绝所有请求".to_string(),
                        conditions: vec![],
                        decision: PolicyDecision::Deny,
                        priority: 0,
                    },
                ],
            },
            PolicyTemplate {
                category: "时间限制".to_string(),
                name: "维护窗口策略".to_string(),
                description: "仅允许维护时间窗口（02:00-04:00）内的写操作".to_string(),
                rules: vec![
                    PolicyRule {
                        id: "maintenance-write".to_string(),
                        name: "维护窗口写操作".to_string(),
                        description: "维护模式（02:00-04:00）内允许写操作".to_string(),
                        conditions: vec![
                            Condition {
                                field: "time_hour".to_string(),
                                operator: Operator::TimeWindow,
                                value: serde_json::json!([2, 4]),
                            },
                            Condition {
                                field: "maintenance_mode".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!(true),
                            },
                            Condition {
                                field: "operation".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("write"),
                            },
                        ],
                        decision: PolicyDecision::Allow,
                        priority: 20,
                    },
                    PolicyRule {
                        id: "default-deny".to_string(),
                        name: "默认拒绝".to_string(),
                        description: "默认拒绝所有请求".to_string(),
                        conditions: vec![],
                        decision: PolicyDecision::Deny,
                        priority: 0,
                    },
                ],
            },
            // === 行为安全 ===
            PolicyTemplate {
                category: "行为安全".to_string(),
                name: "AI 行为降级策略".to_string(),
                description: "当 AI 检测到行为异常评分 > 0.8 时自动拒绝访问".to_string(),
                rules: vec![
                    PolicyRule {
                        id: "behavior-deny".to_string(),
                        name: "行为异常拒绝".to_string(),
                        description: "行为评分超过 0.8 时拒绝访问".to_string(),
                        conditions: vec![
                            Condition {
                                field: "behavior_score".to_string(),
                                operator: Operator::Ge,
                                value: serde_json::json!(0.8),
                            },
                        ],
                        decision: PolicyDecision::Deny,
                        priority: 50,
                    },
                    PolicyRule {
                        id: "behavior-warn".to_string(),
                        name: "行为异常告警".to_string(),
                        description: "行为评分 0.5-0.8 时允许但记录告警".to_string(),
                        conditions: vec![
                            Condition {
                                field: "behavior_score".to_string(),
                                operator: Operator::Ge,
                                value: serde_json::json!(0.5),
                            },
                        ],
                        decision: PolicyDecision::Log,
                        priority: 40,
                    },
                    PolicyRule {
                        id: "default-allow".to_string(),
                        name: "默认允许".to_string(),
                        description: "行为正常时允许访问".to_string(),
                        conditions: vec![],
                        decision: PolicyDecision::Allow,
                        priority: 10,
                    },
                ],
            },
            PolicyTemplate {
                category: "行为安全".to_string(),
                name: "多级行为响应策略".to_string(),
                description: "行为评分分级响应：正常放行 / 轻度异常降级为只读 / 严重异常拒绝".to_string(),
                rules: vec![
                    PolicyRule {
                        id: "behavior-critical-deny".to_string(),
                        name: "严重异常拒绝".to_string(),
                        description: "行为评分 > 0.9 时直接拒绝".to_string(),
                        conditions: vec![
                            Condition {
                                field: "behavior_score".to_string(),
                                operator: Operator::Gt,
                                value: serde_json::json!(0.9),
                            },
                        ],
                        decision: PolicyDecision::Deny,
                        priority: 100,
                    },
                    PolicyRule {
                        id: "behavior-high-deny".to_string(),
                        name: "高度异常拒绝".to_string(),
                        description: "行为评分 0.8-0.9 时拒绝".to_string(),
                        conditions: vec![
                            Condition {
                                field: "behavior_score".to_string(),
                                operator: Operator::Ge,
                                value: serde_json::json!(0.8),
                            },
                        ],
                        decision: PolicyDecision::Deny,
                        priority: 90,
                    },
                    PolicyRule {
                        id: "behavior-medium-log".to_string(),
                        name: "中度异常记录".to_string(),
                        description: "行为评分 0.5-0.8 时允许但记录".to_string(),
                        conditions: vec![
                            Condition {
                                field: "behavior_score".to_string(),
                                operator: Operator::Ge,
                                value: serde_json::json!(0.5),
                            },
                        ],
                        decision: PolicyDecision::Log,
                        priority: 50,
                    },
                    PolicyRule {
                        id: "default-allow".to_string(),
                        name: "正常放行".to_string(),
                        description: "行为评分 < 0.5 时正常允许".to_string(),
                        conditions: vec![],
                        decision: PolicyDecision::Allow,
                        priority: 10,
                    },
                ],
            },
            // === 设备级别 ===
            PolicyTemplate {
                category: "设备级别".to_string(),
                name: "高级设备全权限策略".to_string(),
                description: "A级设备（ARM Linux 工控机）允许全部操作".to_string(),
                rules: vec![
                    PolicyRule {
                        id: "level-a-allow".to_string(),
                        name: "A级设备全权限".to_string(),
                        description: "A级设备允许全部操作".to_string(),
                        conditions: vec![
                            Condition {
                                field: "trust_level".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("A"),
                            },
                        ],
                        decision: PolicyDecision::Allow,
                        priority: 20,
                    },
                    PolicyRule {
                        id: "level-b-readonly".to_string(),
                        name: "B级设备只读".to_string(),
                        description: "B级设备（Cortex-M）仅允许读操作".to_string(),
                        conditions: vec![
                            Condition {
                                field: "trust_level".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("B"),
                            },
                            Condition {
                                field: "operation".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("read"),
                            },
                        ],
                        decision: PolicyDecision::Allow,
                        priority: 15,
                    },
                    PolicyRule {
                        id: "level-c-allow".to_string(),
                        name: "C级设备有限访问".to_string(),
                        description: "C级设备（ESP32）仅允许 MQTT 遥测上报".to_string(),
                        conditions: vec![
                            Condition {
                                field: "trust_level".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("C"),
                            },
                            Condition {
                                field: "protocol".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("mqtt"),
                            },
                            Condition {
                                field: "operation".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("publish"),
                            },
                        ],
                        decision: PolicyDecision::Allow,
                        priority: 15,
                    },
                    PolicyRule {
                        id: "default-deny".to_string(),
                        name: "默认拒绝".to_string(),
                        description: "默认拒绝所有请求".to_string(),
                        conditions: vec![],
                        decision: PolicyDecision::Deny,
                        priority: 0,
                    },
                ],
            },
            // === 综合策略 ===
            PolicyTemplate {
                category: "综合策略".to_string(),
                name: "生产环境综合策略".to_string(),
                description: "包含默认拒绝、Modbus 读允许、MQTT 遥测允许、行为异常降级、维护窗口的完整策略".to_string(),
                rules: vec![
                    PolicyRule {
                        id: "behavior-deny".to_string(),
                        name: "行为异常拒绝".to_string(),
                        description: "行为评分超过 0.8 时拒绝访问".to_string(),
                        conditions: vec![Condition {
                            field: "behavior_score".to_string(),
                            operator: Operator::Ge,
                            value: serde_json::json!(0.8),
                        }],
                        decision: PolicyDecision::Deny,
                        priority: 50,
                    },
                    PolicyRule {
                        id: "modbus-read-allow".to_string(),
                        name: "允许Modbus读操作".to_string(),
                        description: "允许设备的 Modbus 读操作（功能码 3/4）".to_string(),
                        conditions: vec![
                            Condition {
                                field: "protocol".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("modbus"),
                            },
                            Condition {
                                field: "operation".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("read"),
                            },
                            Condition {
                                field: "function_code".to_string(),
                                operator: Operator::In,
                                value: serde_json::json!([3, 4]),
                            },
                        ],
                        decision: PolicyDecision::Allow,
                        priority: 10,
                    },
                    PolicyRule {
                        id: "mqtt-publish-allow".to_string(),
                        name: "允许MQTT发布".to_string(),
                        description: "允许设备向指定 topic 发布 MQTT 消息".to_string(),
                        conditions: vec![
                            Condition {
                                field: "protocol".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("mqtt"),
                            },
                            Condition {
                                field: "operation".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("publish"),
                            },
                            Condition {
                                field: "topic".to_string(),
                                operator: Operator::Regex,
                                value: serde_json::json!("^sensors/.+/telemetry$"),
                            },
                        ],
                        decision: PolicyDecision::Allow,
                        priority: 10,
                    },
                    PolicyRule {
                        id: "maintenance-write".to_string(),
                        name: "维护窗口写操作".to_string(),
                        description: "维护模式（02:00-04:00）内允许写操作".to_string(),
                        conditions: vec![
                            Condition {
                                field: "time_hour".to_string(),
                                operator: Operator::TimeWindow,
                                value: serde_json::json!([2, 4]),
                            },
                            Condition {
                                field: "maintenance_mode".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!(true),
                            },
                            Condition {
                                field: "operation".to_string(),
                                operator: Operator::Eq,
                                value: serde_json::json!("write"),
                            },
                        ],
                        decision: PolicyDecision::Allow,
                        priority: 20,
                    },
                    PolicyRule {
                        id: "default-deny".to_string(),
                        name: "默认拒绝".to_string(),
                        description: "默认拒绝所有请求".to_string(),
                        conditions: vec![],
                        decision: PolicyDecision::Deny,
                        priority: 0,
                    },
                ],
            },
        ]
    }
}

/// 策略模板
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PolicyTemplate {
    /// 分类
    pub category: String,
    /// 模板名称
    pub name: String,
    /// 模板描述
    pub description: String,
    /// 规则列表
    pub rules: Vec<PolicyRule>,
}
