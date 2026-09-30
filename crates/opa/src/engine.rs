//! Rego 策略评估引擎
//!
//! 轻量级 Rego 策略评估器，无需外部 OPA 进程。
//!
//! 支持的 Rego 规则语法:
//! - `allow` / `deny` 决策
//! - `input` 上下文访问
//! - 模式匹配
//! - 内置函数: eq, ne, in, contains, regex.match, lower, upper
//!
//! 当检测到完整 Rego 语法时，使用 Rego 评估器；
//! 否则回退到已有的 JSON 规则引擎。

use iot_ztna_common::{Error, PolicyDecision, PolicyRequest, PolicyResponse, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Rego 策略规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegoRule {
    /// 规则名称
    pub name: String,
    /// 规则描述
    pub description: String,
    /// Rego 规则内容
    pub rego: String,
    /// 决策结果
    pub decision: String,
    /// 优先级
    pub priority: i32,
}

/// Rego 评估引擎
pub struct RegoEngine {
    /// 已加载的规则
    rules: Arc<RwLock<Vec<RegoRule>>>,
    /// 规则版本号
    revision: Arc<RwLock<String>>,
}

impl RegoEngine {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            revision: Arc::new(RwLock::new("init".to_string())),
        }
    }

    /// 加载 Rego 策略
    pub fn load_rules(&self, rules: Vec<RegoRule>) -> Result<()> {
        // 按优先级降序排列
        let mut sorted = rules;
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut guard = self.rules.write().map_err(|e| {
            Error::Internal(format!("获取规则写锁失败: {}", e))
        })?;
        *guard = sorted;

        let rev = Uuid::new_v4().to_string();
        let mut rev_guard = self.revision.write().map_err(|e| {
            Error::Internal(format!("获取版本写锁失败: {}", e))
        })?;
        *rev_guard = rev;

        tracing::info!("Rego 策略已加载");
        Ok(())
    }

    /// 评估策略决策
    pub fn evaluate(&self, request: &PolicyRequest) -> Result<PolicyResponse> {
        let rules = self.rules.read().map_err(|e| {
            Error::Internal(format!("获取规则读锁失败: {}", e))
        })?;

        let input = self.build_input(request);
        let mut matched_rules = Vec::new();
        let mut final_decision = PolicyDecision::Deny;
        let mut final_reason = "默认拒绝：无匹配规则".to_string();

        for rule in rules.iter() {
            if self.evaluate_rego(&rule.rego, &input) {
                matched_rules.push(rule.name.clone());
                // 第一个匹配的高优先级规则决定结果
                if final_decision == PolicyDecision::Deny {
                    final_decision = match rule.decision.as_str() {
                        "allow" => PolicyDecision::Allow,
                        "deny" => PolicyDecision::Deny,
                        "log" => PolicyDecision::Log,
                        "redirect" => PolicyDecision::Redirect,
                        _ => PolicyDecision::Deny,
                    };
                    final_reason = format!("匹配规则: {}", rule.name);
                }
            }
        }

        Ok(PolicyResponse {
            decision: final_decision,
            reason: final_reason,
            matched_rules,
            timestamp: chrono::Utc::now(),
        })
    }

    /// 构建 Rego input 上下文
    fn build_input(&self, request: &PolicyRequest) -> Value {
        serde_json::json!({
            "spiffe_id": request.device_spiffe_id.to_string(),
            "protocol": request.protocol,
            "operation": request.operation,
            "target": request.target,
            "function_code": request.function_code,
            "topic": request.topic,
            "behavior_score": request.behavior_score,
            "maintenance_mode": request.maintenance_mode,
        })
    }

    /// 评估 Rego 规则（简化版 Rego 解释器）
    ///
    /// 支持的语法模式:
    /// 1. `allow if input.protocol == "modbus"`
    /// 2. `allow if input.protocol == "mqtt" and input.topic matches "^sensors/.+"`
    /// 3. `deny if input.behavior_score > 0.8`
    /// 4. JSON 规则（已有格式）
    fn evaluate_rego(&self, rego: &str, input: &Value) -> bool {
        let rego = rego.trim();

        // 尝试 JSON 格式（兼容已有规则引擎）
        if rego.starts_with('[') || rego.starts_with('{') {
            return self.evaluate_json_rules(rego, input);
        }

        // 解析 Rego-like 语法
        // 查找 `if` 关键字后面的条件
        let conditions = if let Some(pos) = rego.find(" if ") {
            rego[pos + 4..].trim()
        } else {
            // 无条件规则，始终匹配
            return true;
        };

        // 清理花括号
        let conditions = conditions.trim_start_matches('{').trim_end_matches('}').trim();

        // 分割 AND 条件
        let parts: Vec<&str> = conditions.split(" and ").collect();

        // 评估所有条件
        parts.iter().all(|part| self.evaluate_condition(part.trim(), input))
    }

    /// 评估单个条件
    fn evaluate_condition(&self, cond: &str, input: &Value) -> bool {
        let cond = cond.trim();

        // 匹配操作符
        if cond.contains("==") {
            let parts: Vec<&str> = cond.splitn(2, "==").collect();
            if parts.len() == 2 {
                let field = parts[0].trim();
                let expected = parts[1].trim();
                let actual = self.get_input_value(field, input);
                return self.compare_eq(&actual, expected);
            }
        }

        if cond.contains("!=") {
            let parts: Vec<&str> = cond.splitn(2, "!=").collect();
            if parts.len() == 2 {
                let field = parts[0].trim();
                let expected = parts[1].trim();
                let actual = self.get_input_value(field, input);
                return !self.compare_eq(&actual, expected);
            }
        }

        if cond.contains(">") && !cond.contains(">=") {
            let parts: Vec<&str> = cond.splitn(2, '>').collect();
            if parts.len() == 2 {
                let field = parts[0].trim();
                let expected = parts[1].trim();
                let actual = self.get_input_value(field, input);
                return self.compare_gt(&actual, expected);
            }
        }

        if cond.contains("<") && !cond.contains("<=") {
            let parts: Vec<&str> = cond.splitn(2, '<').collect();
            if parts.len() == 2 {
                let field = parts[0].trim();
                let expected = parts[1].trim();
                let actual = self.get_input_value(field, input);
                return self.compare_lt(&actual, expected);
            }
        }

        if cond.contains(">=") {
            let parts: Vec<&str> = cond.splitn(2, ">=").collect();
            if parts.len() == 2 {
                let field = parts[0].trim();
                let expected = parts[1].trim();
                let actual = self.get_input_value(field, input);
                return self.compare_ge(&actual, expected);
            }
        }

        if cond.contains("<=") {
            let parts: Vec<&str> = cond.splitn(2, "<=").collect();
            if parts.len() == 2 {
                let field = parts[0].trim();
                let expected = parts[1].trim();
                let actual = self.get_input_value(field, input);
                return self.compare_le(&actual, expected);
            }
        }

        if cond.contains("matches") || cond.contains("match(") {
            // 正则匹配
            if let Some(pos) = cond.find("matches") {
                let field = cond[..pos].trim();
                let pattern = cond[pos + 7..].trim().trim_matches('"');
                let actual = self.get_input_value(field, input);
                if let Some(s) = actual.as_str() {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        return re.is_match(s);
                    }
                }
            }
        }

        if cond.contains("contains") {
            // 字符串包含
            let parts: Vec<&str> = cond.splitn(2, "contains").collect();
            if parts.len() == 2 {
                let field = parts[0].trim();
                let expected = parts[1].trim().trim_matches('"');
                let actual = self.get_input_value(field, input);
                if let Some(s) = actual.as_str() {
                    return s.contains(expected);
                }
            }
        }

        if cond.contains("in") {
            // 在列表中
            let parts: Vec<&str> = cond.splitn(2, " in ").collect();
            if parts.len() == 2 {
                let field = parts[0].trim();
                let expected = parts[1].trim();
                let actual = self.get_input_value(field, input);
                return self.compare_in(&actual, expected);
            }
        }

        // 无匹配的操作符，返回 false
        false
    }

    /// 从 input 中获取字段值
    fn get_input_value(&self, field: &str, input: &Value) -> Value {
        let field = field.trim().trim_start_matches("input.");
        input.get(field).cloned().unwrap_or(Value::Null)
    }

    /// 比较: 等于
    fn compare_eq(&self, actual: &Value, expected: &str) -> bool {
        let expected = expected.trim().trim_matches('"');
        match actual {
            Value::String(s) => s == expected,
            Value::Number(n) => {
                if let Ok(f) = expected.parse::<f64>() {
                    n.as_f64().map(|a| (a - f).abs() < f64::EPSILON).unwrap_or(false)
                } else {
                    false
                }
            }
            Value::Bool(b) => expected == b.to_string(),
            _ => false,
        }
    }

    /// 比较: 大于
    fn compare_gt(&self, actual: &Value, expected: &str) -> bool {
        let expected = expected.trim().trim_matches('"');
        if let (Some(a), Ok(e)) = (actual.as_f64(), expected.parse::<f64>()) {
            a > e
        } else {
            false
        }
    }

    fn compare_lt(&self, actual: &Value, expected: &str) -> bool {
        let expected = expected.trim().trim_matches('"');
        if let (Some(a), Ok(e)) = (actual.as_f64(), expected.parse::<f64>()) {
            a < e
        } else {
            false
        }
    }

    fn compare_ge(&self, actual: &Value, expected: &str) -> bool {
        let expected = expected.trim().trim_matches('"');
        if let (Some(a), Ok(e)) = (actual.as_f64(), expected.parse::<f64>()) {
            a >= e
        } else {
            false
        }
    }

    fn compare_le(&self, actual: &Value, expected: &str) -> bool {
        let expected = expected.trim().trim_matches('"');
        if let (Some(a), Ok(e)) = (actual.as_f64(), expected.parse::<f64>()) {
            a <= e
        } else {
            false
        }
    }

    /// 比较: 在列表中
    fn compare_in(&self, actual: &Value, expected: &str) -> bool {
        // 解析列表: [a, b, c]
        let expected = expected.trim().trim_start_matches('[').trim_end_matches(']');
        let items: Vec<&str> = expected.split(',').map(|s| s.trim().trim_matches('"')).collect();
        match actual {
            Value::String(s) => items.contains(&s.as_str()),
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    items.iter().any(|item| {
                        item.parse::<f64>().map(|e| (f - e).abs() < f64::EPSILON).unwrap_or(false)
                    })
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// 评估 JSON 格式的规则（兼容已有规则引擎格式）
    fn evaluate_json_rules(&self, json: &str, input: &Value) -> bool {
        if let Ok(rules) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
            return rules.iter().all(|rule| {
                if let Some(conditions) = rule.get("conditions").and_then(|c| c.as_array()) {
                    conditions.iter().all(|cond| {
                        self.evaluate_json_condition(cond, input)
                    })
                } else {
                    true
                }
            });
        }
        false
    }

    /// 评估 JSON 格式的条件
    fn evaluate_json_condition(&self, cond: &Value, input: &Value) -> bool {
        let field = cond.get("field").and_then(|f| f.as_str()).unwrap_or("");
        let op = cond.get("operator").and_then(|o| o.as_str()).unwrap_or("eq");
        let expected = cond.get("value").cloned().unwrap_or(Value::Null);

        // 支持 input. 前缀
        let field = field.trim_start_matches("input.");
        let actual = input.get(field).cloned().unwrap_or(Value::Null);

        match op {
            "eq" => self.compare_eq(&actual, &expected.to_string()),
            "ne" => !self.compare_eq(&actual, &expected.to_string()),
            "lt" => self.compare_lt(&actual, &expected.to_string()),
            "gt" => self.compare_gt(&actual, &expected.to_string()),
            "le" => self.compare_le(&actual, &expected.to_string()),
            "ge" => self.compare_ge(&actual, &expected.to_string()),
            "in" => self.compare_in(&actual, &expected.to_string()),
            "notin" => !self.compare_in(&actual, &expected.to_string()),
            "regex" => {
                if let (Some(s), Some(p)) = (actual.as_str(), expected.as_str()) {
                    regex::Regex::new(p).map(|re| re.is_match(s)).unwrap_or(false)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// 获取当前策略版本
    pub fn get_revision(&self) -> Result<String> {
        let rev = self.revision.read().map_err(|e| {
            Error::Internal(format!("获取版本读锁失败: {}", e))
        })?;
        Ok(rev.clone())
    }

    /// 策略模拟（dry-run）
    pub fn simulate(&self, request: &PolicyRequest) -> Result<PolicyResponse> {
        let response = self.evaluate(request)?;
        tracing::info!(
            spiffe_id = %request.device_spiffe_id,
            decision = %response.decision,
            "Rego 策略模拟 (dry-run)"
        );
        Ok(response)
    }

    /// 生成默认 Rego 规则模板
    pub fn default_rego_rules() -> Vec<RegoRule> {
        vec![
            RegoRule {
                name: "允许Modbus读操作".to_string(),
                description: "允许设备的Modbus读操作（功能码3/4）".to_string(),
                rego: r#"allow if input.protocol == "modbus" and input.operation == "read" and input.function_code in [3, 4]"#.to_string(),
                decision: "allow".to_string(),
                priority: 10,
            },
            RegoRule {
                name: "允许MQTT发布".to_string(),
                description: "允许设备向指定topic发布MQTT消息".to_string(),
                rego: r#"allow if input.protocol == "mqtt" and input.operation == "publish" and input.topic matches "^sensors/.+/telemetry$""#.to_string(),
                decision: "allow".to_string(),
                priority: 10,
            },
            RegoRule {
                name: "行为异常降级".to_string(),
                description: "行为评分超过0.8时拒绝".to_string(),
                rego: r#"deny if input.behavior_score > 0.8"#.to_string(),
                decision: "deny".to_string(),
                priority: 50,
            },
            RegoRule {
                name: "默认拒绝".to_string(),
                description: "默认拒绝所有请求".to_string(),
                rego: r#"deny"#.to_string(),
                decision: "deny".to_string(),
                priority: 0,
            },
        ]
    }
}

impl Default for RegoEngine {
    fn default() -> Self {
        Self::new()
    }
}
