//! 策略管理器
//!
//! 统一管理策略生命周期: 创建 → 模拟 → 审核 → 发布 → 执行 → 审计

use crate::engine::{PolicyEngine, PolicyRule};
use crate::executor::PolicyExecutor;
use iot_ztna_common::{
    Policy, PolicyDecision, PolicyRequest, PolicyResponse, PolicyStatus, Result,
};
use iot_ztna_storage::Database;
use std::sync::Arc;

/// 策略管理器
pub struct PolicyManager {
    engine: PolicyEngine,
    executor: Arc<dyn PolicyExecutor>,
    db: Database,
}

impl PolicyManager {
    pub fn new(db: Database, executor: Arc<dyn PolicyExecutor>) -> Self {
        let engine = PolicyEngine::new(db.clone());
        Self {
            engine,
            executor,
            db,
        }
    }

    /// 初始化（加载已有策略 + 初始化执行器）
    pub fn init(&self) -> Result<()> {
        self.engine.load_policies()?;
        Ok(())
    }

    /// 创建策略
    pub fn create_policy(
        &self,
        name: &str,
        rules: Vec<PolicyRule>,
        created_by: &str,
    ) -> Result<String> {
        self.engine.create_policy(name, rules, created_by)
    }

    /// 发布策略
    pub fn publish_policy(&self, policy_id: &str) -> Result<()> {
        self.engine.publish_policy(policy_id)
    }

    /// 模拟策略
    pub fn simulate(&self, request: &PolicyRequest) -> Result<PolicyResponse> {
        self.engine.simulate(request)
    }

    /// 评估并执行策略
    pub async fn evaluate_and_execute(
        &self,
        request: &PolicyRequest,
        device_ip: Option<std::net::IpAddr>,
        target_port: Option<u16>,
    ) -> Result<PolicyResponse> {
        let response = self.engine.evaluate(request)?;

        // 执行策略决策
        if let (Some(ip), Some(port)) = (device_ip, target_port) {
            match response.decision {
                PolicyDecision::Allow => {
                    if let Err(e) = self.executor.allow(ip, port).await {
                        tracing::error!(error = %e, "执行允许规则失败");
                    }
                }
                PolicyDecision::Deny => {
                    if let Err(e) = self.executor.deny(ip).await {
                        tracing::error!(error = %e, "执行拒绝规则失败");
                    }
                }
                _ => {}
            }
        }

        Ok(response)
    }

    /// 列出策略
    pub fn list_policies(&self, limit: u32, offset: u32) -> Result<Vec<Policy>> {
        self.db.list_policies(limit, offset)
    }

    /// 获取策略
    pub fn get_policy(&self, id: &str) -> Result<Option<Policy>> {
        self.db.get_policy(id)
    }

    /// 删除策略
    pub fn delete_policy(&self, id: &str) -> Result<()> {
        self.db.delete_policy(id)
    }

    /// 归档策略
    pub fn archive_policy(&self, id: &str) -> Result<()> {
        self.db.update_policy_status(id, &PolicyStatus::Archived)?;
        self.engine.load_policies()?;
        Ok(())
    }

    /// 引擎引用
    pub fn engine(&self) -> &PolicyEngine {
        &self.engine
    }
}
