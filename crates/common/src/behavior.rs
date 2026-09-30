//! 行为基线检测引擎
//!
//! 基于统计模型的轻量级行为异常检测：
//! - 滑动窗口频率统计
//! - Z-Score 异常检测
//! - 时间窗口模式匹配
//! - 未知目标检测

use crate::types::*;
use crate::{Error, Result, SpiffeId};
use chrono::{Duration, Timelike, Utc};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 行为检测引擎
pub struct BehaviorEngine {
    /// 设备行为历史（滑动窗口）
    histories: Arc<RwLock<HashMap<String, Vec<BehaviorEvent>>>>,
    /// 设备行为基线
    baselines: Arc<RwLock<HashMap<String, BehaviorBaseline>>>,
    /// 滑动窗口大小（秒）
    window_size: i64,
    /// 最小样本数（不足时不评估）
    min_samples: usize,
    /// 异常评分阈值
    anomaly_threshold: f64,
}

impl BehaviorEngine {
    pub fn new() -> Self {
        Self {
            histories: Arc::new(RwLock::new(HashMap::new())),
            baselines: Arc::new(RwLock::new(HashMap::new())),
            window_size: 3600,       // 1 小时窗口
            min_samples: 10,          // 至少 10 个样本
            anomaly_threshold: 0.7,  // 异常阈值
        }
    }

    /// 记录行为事件
    pub fn record_event(&self, event: BehaviorEvent) -> Result<()> {
        let mut histories = self.histories.write().map_err(|e| {
            Error::Internal(format!("获取行为历史写锁失败: {}", e))
        })?;

        let spiffe = event.spiffe_id.to_string();
        let history = histories.entry(spiffe.clone()).or_default();

        // 清理窗口外的旧事件
        let cutoff = Utc::now() - Duration::seconds(self.window_size);
        history.retain(|e| e.timestamp > cutoff);

        // 添加新事件
        history.push(event);

        Ok(())
    }

    /// 评估设备行为异常
    pub fn assess(&self, spiffe_id: &SpiffeId) -> Result<BehaviorAssessment> {
        let spiffe = spiffe_id.to_string();

        let histories = self.histories.read().map_err(|e| {
            Error::Internal(format!("获取行为历史读锁失败: {}", e))
        })?;

        let history = histories.get(&spiffe);

        // 样本不足，返回正常
        if history.is_none() || history.unwrap().len() < self.min_samples {
            return Ok(BehaviorAssessment::default());
        }

        let events = history.unwrap();
        let mut score = 0.0_f64;
        let mut anomaly_types = Vec::new();
        let mut feature_contributions = Vec::new();

        // 1. 频率异常检测（Z-Score）
        let current_rate = self.calculate_rate(events);
        let baselines = self.baselines.read().map_err(|e| {
            Error::Internal(format!("获取基线读锁失败: {}", e))
        })?;

        if let Some(baseline) = baselines.get(&spiffe) {
            // 检查频率异常
            if let Some(expected_rate) = baseline.protocol_rates.get(&events.last().unwrap().protocol) {
                let expected = expected_rate.as_f64().unwrap_or(0.0);
                if expected > 0.0 {
                    let z_score = (current_rate - expected).abs() / expected.sqrt();
                    let freq_contribution = if z_score > 3.0 { 0.3 } else { 0.0 };
                    score += freq_contribution;
                    if z_score > 3.0 {
                        anomaly_types.push("频率异常".to_string());
                    }
                    feature_contributions.push(crate::types::FeatureContribution {
                        feature: "请求频率".to_string(),
                        contribution: freq_contribution,
                        value: format!("{:.2} 次/分", current_rate),
                        baseline_value: format!("{:.2} 次/分", expected),
                    });
                }
            }

            // 2. 检查未知操作
            let recent_ops: Vec<&str> = events.iter().rev().take(20).map(|e| e.operation.as_str()).collect();
            let unknown_ops: Vec<String> = recent_ops.iter()
                .filter(|op| !baseline.common_operations.contains(&op.to_string()))
                .map(|s| s.to_string())
                .collect();
            let ops_contribution = (unknown_ops.len() as f64 * 0.1).min(0.3);
            score += ops_contribution;
            if !unknown_ops.is_empty() {
                anomaly_types.push(format!("未知操作: {}", unknown_ops.iter().take(3).cloned().collect::<Vec<_>>().join(",")));
            }
            feature_contributions.push(crate::types::FeatureContribution {
                feature: "操作类型".to_string(),
                contribution: ops_contribution,
                value: format!("{}/{} 未知", unknown_ops.len(), recent_ops.len()),
                baseline_value: baseline.common_operations.join(","),
            });

            // 3. 检查未知目标
            let recent_targets: Vec<&str> = events.iter().rev().take(20).map(|e| e.target.as_str()).collect();
            let unknown_targets: Vec<String> = recent_targets.iter()
                .filter(|t| !baseline.common_targets.contains(&t.to_string()))
                .map(|s| s.to_string())
                .collect();
            let target_contribution = (unknown_targets.len() as f64 * 0.1).min(0.3);
            score += target_contribution;
            if !unknown_targets.is_empty() {
                anomaly_types.push(format!("未知目标: {}", unknown_targets.iter().take(3).cloned().collect::<Vec<_>>().join(",")));
            }
            feature_contributions.push(crate::types::FeatureContribution {
                feature: "访问目标".to_string(),
                contribution: target_contribution,
                value: format!("{}/{} 未知", unknown_targets.len(), recent_targets.len()),
                baseline_value: baseline.common_targets.join(","),
            });

            // 4. 时间窗口检测
            let now = Utc::now();
            let hour = now.hour();
            let (start, end) = baseline.active_hours;
            let in_active_window = if start <= end {
                hour >= start && hour < end
            } else {
                hour >= start || hour < end
            };
            let time_contribution = if !in_active_window { 0.15 } else { 0.0 };
            score += time_contribution;
            if !in_active_window {
                anomaly_types.push("非活跃时段".to_string());
            }
            feature_contributions.push(crate::types::FeatureContribution {
                feature: "活跃时段".to_string(),
                contribution: time_contribution,
                value: format!("{}:00", hour),
                baseline_value: format!("{}:00-{}:00", start, end),
            });

            // 5. 数据大小异常
            let recent_sizes: Vec<f64> = events.iter().rev().take(20)
                .filter_map(|e| e.data_size.map(|s| s as f64))
                .collect();
            let data_contribution = if !recent_sizes.is_empty() {
                let avg_recent = recent_sizes.iter().sum::<f64>() / recent_sizes.len() as f64;
                let ratio = if baseline.avg_data_size > 0.0 {
                    avg_recent / baseline.avg_data_size
                } else {
                    1.0
                };
                let contrib = if ratio > 3.0 || ratio < 0.33 { 0.2 } else { 0.0 };
                score += contrib;
                if ratio > 3.0 || ratio < 0.33 {
                    anomaly_types.push("数据量异常".to_string());
                }
                feature_contributions.push(crate::types::FeatureContribution {
                    feature: "数据大小".to_string(),
                    contribution: contrib,
                    value: format!("{:.0} bytes", avg_recent),
                    baseline_value: format!("{:.0} bytes", baseline.avg_data_size),
                });
                contrib
            } else {
                0.0
            };
            let _ = data_contribution;
        }

        // 限制评分在 0-1 范围
        let score = score.min(1.0);
        let is_anomaly = score >= self.anomaly_threshold;

        let recommended_action = if is_anomaly {
            PolicyDecision::Deny
        } else if score > self.anomaly_threshold * 0.7 {
            PolicyDecision::Log
        } else {
            PolicyDecision::Allow
        };

        let reason = if is_anomaly {
            format!("行为异常评分 {:.2}: {}", score, anomaly_types.join(", "))
        } else if score > 0.0 {
            format!("轻微异常评分 {:.2}: {}", score, anomaly_types.join(", "))
        } else {
            "正常行为".to_string()
        };

        Ok(BehaviorAssessment {
            anomaly_score: score,
            is_anomaly,
            anomaly_types,
            recommended_action,
            reason,
            feature_contributions,
        })
    }

    /// 计算当前请求频率（次/分钟）
    fn calculate_rate(&self, events: &[BehaviorEvent]) -> f64 {
        if events.is_empty() {
            return 0.0;
        }
        let window_minutes = self.window_size as f64 / 60.0;
        events.len() as f64 / window_minutes
    }

    /// 更新设备行为基线
    pub fn update_baseline(&self, spiffe_id: &SpiffeId) -> Result<()> {
        let spiffe = spiffe_id.to_string();

        let histories = self.histories.read().map_err(|e| {
            Error::Internal(format!("获取行为历史读锁失败: {}", e))
        })?;

        let events = match histories.get(&spiffe) {
            Some(e) if e.len() >= self.min_samples => e,
            _ => return Ok(()), // 样本不足，不更新基线
        };

        // 计算协议频率
        let mut protocol_counts: HashMap<String, u32> = HashMap::new();
        let mut operations: HashMap<String, u32> = HashMap::new();
        let mut targets: HashMap<String, u32> = HashMap::new();
        let mut total_data_size: f64 = 0.0;
        let mut data_count: u32 = 0;
        let mut hours: Vec<u32> = Vec::new();

        for event in events {
            *protocol_counts.entry(event.protocol.clone()).or_default() += 1;
            *operations.entry(event.operation.clone()).or_default() += 1;
            *targets.entry(event.target.clone()).or_default() += 1;
            if let Some(size) = event.data_size {
                total_data_size += size as f64;
                data_count += 1;
            }
            hours.push(event.timestamp.hour());
        }

        let window_minutes = self.window_size as f64 / 60.0;
        let protocol_rates: serde_json::Value = serde_json::to_value(
            protocol_counts.iter().map(|(k, v)| {
                (k.clone(), *v as f64 / window_minutes)
            }).collect::<HashMap<String, f64>>()
        )?;

        // 取 Top-5 常用操作和目标
        let mut ops_vec: Vec<(String, u32)> = operations.into_iter().collect();
        ops_vec.sort_by(|a, b| b.1.cmp(&a.1));
        let common_operations = ops_vec.iter().take(5).map(|(k, _)| k.clone()).collect();

        let mut targets_vec: Vec<(String, u32)> = targets.into_iter().collect();
        targets_vec.sort_by(|a, b| b.1.cmp(&a.1));
        let common_targets = targets_vec.iter().take(5).map(|(k, _)| k.clone()).collect();

        // 计算活跃时间段
        let active_hours = if hours.is_empty() {
            (0, 24)
        } else {
            let min_hour = hours.iter().min().unwrap();
            let max_hour = hours.iter().max().unwrap();
            (*min_hour, (*max_hour + 1).min(24))
        };

        let avg_data_size = if data_count > 0 {
            total_data_size / data_count as f64
        } else {
            0.0
        };

        let baseline = BehaviorBaseline {
            spiffe_id: spiffe.clone(),
            protocol_rates,
            common_operations,
            common_targets,
            avg_data_size,
            active_hours,
            anomaly_threshold: self.anomaly_threshold,
            updated_at: Utc::now(),
        };

        let mut baselines = self.baselines.write().map_err(|e| {
            Error::Internal(format!("获取基线写锁失败: {}", e))
        })?;
        baselines.insert(spiffe, baseline);

        Ok(())
    }

    /// 获取设备行为基线
    pub fn get_baseline(&self, spiffe_id: &SpiffeId) -> Result<Option<BehaviorBaseline>> {
        let baselines = self.baselines.read().map_err(|e| {
            Error::Internal(format!("获取基线读锁失败: {}", e))
        })?;
        Ok(baselines.get(&spiffe_id.to_string()).cloned())
    }

    /// 清理过期行为历史
    pub fn cleanup(&self) -> Result<usize> {
        let cutoff = Utc::now() - Duration::seconds(self.window_size);
        let mut histories = self.histories.write().map_err(|e| {
            Error::Internal(format!("获取行为历史写锁失败: {}", e))
        })?;

        let mut removed = 0;
        for events in histories.values_mut() {
            let before = events.len();
            events.retain(|e| e.timestamp > cutoff);
            removed += before - events.len();
        }

        // 清理空设备
        histories.retain(|_, events| !events.is_empty());

        Ok(removed)
    }

    /// 获取行为统计
    pub fn get_stats(&self) -> Result<BehaviorStats> {
        let histories = self.histories.read().map_err(|e| {
            Error::Internal(format!("获取行为历史读锁失败: {}", e))
        })?;
        let baselines = self.baselines.read().map_err(|e| {
            Error::Internal(format!("获取基线读锁失败: {}", e))
        })?;

        let total_events: usize = histories.values().map(|e| e.len()).sum();
        let devices_with_history = histories.len();
        let devices_with_baseline = baselines.len();

        Ok(BehaviorStats {
            total_events,
            devices_with_history,
            devices_with_baseline,
        })
    }
}

/// 行为统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorStats {
    pub total_events: usize,
    pub devices_with_history: usize,
    pub devices_with_baseline: usize,
}

impl Default for BehaviorEngine {
    fn default() -> Self {
        Self::new()
    }
}
