//! Autoencoder 行为基线异常检测引擎
//!
//! 非对称自编码器实现：
//! - 编码器: 128 → 64 → 32 → 16 (LeakyReLU)
//! - 解码器: 16 → 32 → 64 → 128 (Sigmoid)
//! - 重建误差驱动异常评分
//! - 校准机制：中位数(下锚) → 99.5百分位(上锚) 线性映射
//!
//! 这是纯 Rust 实现，无需外部 ONNX Runtime 依赖，
//! 适合边缘场景的轻量级部署。

use chrono::{DateTime, Utc};
use iot_ztna_common::{
    BehaviorAssessment, BehaviorBaseline, BehaviorEvent, FeatureContribution,
    PolicyDecision,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Autoencoder 引擎
pub struct AutoencoderEngine {
    /// 模型权重
    model: Arc<RwLock<AutoencoderModel>>,
    /// 设备行为历史（滑动窗口）
    histories: Arc<RwLock<HashMap<String, Vec<BehaviorEvent>>>>,
    /// 设备行为基线
    baselines: Arc<RwLock<HashMap<String, BehaviorBaseline>>>,
    /// 重建误差校准参数
    calibration: Arc<RwLock<CalibrationParams>>,
    /// 滑动窗口大小（秒）
    window_size: i64,
    /// 最小样本数
    min_samples: usize,
    /// 异常评分阈值
    anomaly_threshold: f64,
}

/// 校准参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationParams {
    /// 下锚：重建误差中位数（异常评分 = 0）
    pub lower_anchor: f64,
    /// 上锚：重建误差 99.5 百分位（异常评分 = 1）
    pub upper_anchor: f64,
    /// 校准样本数
    pub calibration_samples: usize,
    /// 校准时间
    pub calibrated_at: DateTime<Utc>,
}

impl Default for CalibrationParams {
    fn default() -> Self {
        Self {
            lower_anchor: 0.01,
            upper_anchor: 0.15,
            calibration_samples: 0,
            calibrated_at: Utc::now(),
        }
    }
}

/// Autoencoder 模型权重
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoencoderModel {
    /// 模型版本
    pub version: String,
    /// 编码器权重矩阵
    pub encoder_weights: Vec<Vec<Vec<f64>>>,
    /// 编码器偏置
    pub encoder_biases: Vec<Vec<f64>>,
    /// 解码器权重矩阵
    pub decoder_weights: Vec<Vec<Vec<f64>>>,
    /// 解码器偏置
    pub decoder_biases: Vec<Vec<f64>>,
    /// 输入维度
    pub input_dim: usize,
    /// 隐藏层维度
    pub hidden_dims: Vec<usize>,
    /// 潜在空间维度
    pub latent_dim: usize,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 训练轮次
    pub epochs: usize,
}

impl AutoencoderModel {
    /// 创建默认初始化模型（Xavier 初始化）
    pub fn new_default() -> Self {
        let input_dim = 128;
        let hidden_dims = vec![64, 32];
        let latent_dim = 16;

        // 编码器: 128→64→32→16
        let mut encoder_weights = Vec::new();
        let mut encoder_biases = Vec::new();

        let dims: Vec<usize> = std::iter::once(input_dim)
            .chain(hidden_dims.iter().copied())
            .chain(std::iter::once(latent_dim))
            .collect();

        for i in 0..dims.len() - 1 {
            let in_dim = dims[i];
            let out_dim = dims[i + 1];
            let limit = (6.0 / (in_dim + out_dim) as f64).sqrt();
            let weights: Vec<Vec<f64>> = (0..out_dim)
                .map(|_| {
                    (0..in_dim)
                        .map(|_| {
                            // 伪随机初始化（实际用 rand）
                            (rand::random::<f64>() - 0.5) * 2.0 * limit
                        })
                        .collect()
                })
                .collect();
            let biases: Vec<f64> = (0..out_dim).map(|_| 0.0).collect();
            encoder_weights.push(weights);
            encoder_biases.push(biases);
        }

        // 解码器: 16→32→64→128 (对称镜像)
        let mut decoder_weights = Vec::new();
        let mut decoder_biases = Vec::new();

        let dec_dims: Vec<usize> = std::iter::once(latent_dim)
            .chain(hidden_dims.iter().rev().copied())
            .chain(std::iter::once(input_dim))
            .collect();

        for i in 0..dec_dims.len() - 1 {
            let in_dim = dec_dims[i];
            let out_dim = dec_dims[i + 1];
            let limit = (6.0 / (in_dim + out_dim) as f64).sqrt();
            let weights: Vec<Vec<f64>> = (0..out_dim)
                .map(|_| {
                    (0..in_dim)
                        .map(|_| (rand::random::<f64>() - 0.5) * 2.0 * limit)
                        .collect()
                })
                .collect();
            let biases: Vec<f64> = (0..out_dim).map(|_| 0.0).collect();
            decoder_weights.push(weights);
            decoder_biases.push(biases);
        }

        Self {
            version: "1.0.0".to_string(),
            encoder_weights,
            encoder_biases,
            decoder_weights,
            decoder_biases,
            input_dim,
            hidden_dims,
            latent_dim,
            created_at: Utc::now(),
            epochs: 0,
        }
    }

    /// 前向传播：编码 → 解码 → 重建
    pub fn forward(&self, input: &[f64]) -> (Vec<f64>, Vec<f64>) {
        // 编码
        let latent = self.encode(input);
        // 解码
        let reconstructed = self.decode(&latent);
        (reconstructed, latent)
    }

    /// 编码器前向传播
    pub fn encode(&self, input: &[f64]) -> Vec<f64> {
        let mut activation = input.to_vec();

        for (layer_idx, (weights, biases)) in self
            .encoder_weights
            .iter()
            .zip(self.encoder_biases.iter())
            .enumerate()
        {
            let mut new_activation = Vec::with_capacity(biases.len());
            for (neuron_idx, (w, b)) in weights.iter().zip(biases.iter()).enumerate() {
                let z: f64 = w
                    .iter()
                    .zip(activation.iter())
                    .map(|(wi, ai)| wi * ai)
                    .sum::<f64>()
                    + b;
                // LeakyReLU for encoder (last layer uses linear for latent space)
                let a = if layer_idx < self.encoder_weights.len() - 1 {
                    leaky_relu(z)
                } else {
                    z // 潜在空间使用线性激活
                };
                new_activation.push(a);
            }
            activation = new_activation;
        }

        activation
    }

    /// 解码器前向传播
    fn decode(&self, latent: &[f64]) -> Vec<f64> {
        let mut activation = latent.to_vec();

        for (layer_idx, (weights, biases)) in self
            .decoder_weights
            .iter()
            .zip(self.decoder_biases.iter())
            .enumerate()
        {
            let mut new_activation = Vec::with_capacity(biases.len());
            for (w, b) in weights.iter().zip(biases.iter()) {
                let z: f64 = w
                    .iter()
                    .zip(activation.iter())
                    .map(|(wi, ai)| wi * ai)
                    .sum::<f64>()
                    + b;
                // Sigmoid for decoder output layer, LeakyReLU for hidden
                let a = if layer_idx < self.decoder_weights.len() - 1 {
                    leaky_relu(z)
                } else {
                    sigmoid(z)
                };
                new_activation.push(a);
            }
            activation = new_activation;
        }

        activation
    }

    /// 计算重建误差（MSE）
    pub fn reconstruction_error(&self, input: &[f64]) -> f64 {
        let (reconstructed, _) = self.forward(input);
        let mse: f64 = input
            .iter()
            .zip(reconstructed.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            / input.len() as f64;
        mse
    }
}

impl AutoencoderEngine {
    pub fn new() -> Self {
        Self {
            model: Arc::new(RwLock::new(AutoencoderModel::new_default())),
            histories: Arc::new(RwLock::new(HashMap::new())),
            baselines: Arc::new(RwLock::new(HashMap::new())),
            calibration: Arc::new(RwLock::new(CalibrationParams::default())),
            window_size: 3600,
            min_samples: 10,
            anomaly_threshold: 0.7,
        }
    }

    /// 提取行为特征向量（128维）
    ///
    /// 特征编码方案：
    /// - 协议 one-hot (4维): modbus/mqtt/opcua/http
    /// - 操作 one-hot (4维): read/write/publish/subscribe
    /// - 小时 sin/cos (2维)
    /// - 频率分桶 (16维)
    /// - 数据大小分桶 (16维)
    /// - 目标地址哈希 (32维)
    /// - 功能码 one-hot (16维)
    /// - 统计特征 (38维): 均值/方差/最大/最小/分位数
    pub fn extract_features(events: &[BehaviorEvent]) -> Vec<f64> {
        let mut features = vec![0.0f64; 128];
        let n = events.len() as f64;

        if n == 0.0 {
            return features;
        }

        // 1. 协议分布 (0-3)
        for event in events {
            match event.protocol.as_str() {
                "modbus" => features[0] += 1.0,
                "mqtt" => features[1] += 1.0,
                "opcua" => features[2] += 1.0,
                "http" => features[3] += 1.0,
                _ => {}
            }
        }
        for i in 0..4 {
            features[i] /= n;
        }

        // 2. 操作分布 (4-7)
        for event in events {
            match event.operation.as_str() {
                "read" => features[4] += 1.0,
                "write" => features[5] += 1.0,
                "publish" => features[6] += 1.0,
                "subscribe" => features[7] += 1.0,
                _ => {}
            }
        }
        for i in 4..8 {
            features[i] /= n;
        }

        // 3. 时间特征 sin/cos (8-9)
        let hours: Vec<f64> = events.iter().map(|e| e.timestamp.format("%H").to_string().parse::<f64>().unwrap_or(0.0)).collect();
        let avg_hour = hours.iter().sum::<f64>() / n;
        features[8] = (avg_hour * std::f64::consts::PI / 12.0).sin();
        features[9] = (avg_hour * std::f64::consts::PI / 12.0).cos();

        // 4. 频率分桶 (10-25)
        let rate = n / (3600.0 / 60.0); // 次/分
        for i in 0..16 {
            let threshold = (i + 1) as f64 * 5.0; // 0, 5, 10, ... 80 次/分
            if rate <= threshold && features[10 + i] == 0.0 {
                features[10 + i] = 1.0;
            }
        }

        // 5. 数据大小分桶 (26-41)
        let sizes: Vec<f64> = events.iter().filter_map(|e| e.data_size.map(|s| s as f64)).collect();
        let avg_size = if !sizes.is_empty() {
            sizes.iter().sum::<f64>() / sizes.len() as f64
        } else {
            0.0
        };
        for i in 0..16 {
            let threshold = (i + 1) as f64 * 1024.0; // 0, 1K, 2K, ... 16K
            if avg_size <= threshold && features[26 + i] == 0.0 {
                features[26 + i] = 1.0;
                break;
            }
        }

        // 6. 目标地址哈希 (42-73)
        for event in events {
            let hash = simple_hash(&event.target);
            let idx = (hash % 32) as usize;
            features[42 + idx] += 1.0;
        }
        for i in 42..74 {
            features[i] /= n;
        }

        // 7. 功能码分布 (74-89)
        for event in events {
            if let Some(fc) = event.function_code {
                let idx = 74 + (fc as usize).min(15);
                features[idx] += 1.0;
            }
        }
        for i in 74..90 {
            features[i] /= n;
        }

        // 8. 统计特征 (90-127)
        // 频率统计
        features[90] = rate; // 频率
        features[91] = if !hours.is_empty() {
            hours.iter().map(|h| (h - avg_hour).powi(2)).sum::<f64>() / n
        } else { 0.0 }; // 频率方差
        features[92] = hours.iter().fold(0.0f64, |a, &b| a.max(b)); // 最大频率
        features[93] = hours.iter().fold(f64::MAX, |a, &b| a.min(b)); // 最小频率

        // 数据大小统计
        features[94] = avg_size; // 平均大小
        features[95] = if !sizes.is_empty() {
            let mean = avg_size;
            sizes.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / sizes.len() as f64
        } else { 0.0 }; // 大小方差
        features[96] = sizes.iter().fold(0.0f64, |a, &b| a.max(b)); // 最大大小
        features[97] = sizes.iter().fold(f64::MAX, |a, &b| a.min(b)); // 最小大小

        // 唯一目标数
        let unique_targets: std::collections::HashSet<&str> =
            events.iter().map(|e| e.target.as_str()).collect();
        features[98] = unique_targets.len() as f64;

        // 唯一操作数
        let unique_ops: std::collections::HashSet<&str> =
            events.iter().map(|e| e.operation.as_str()).collect();
        features[99] = unique_ops.len() as f64;

        // 剩余维度用扩展统计填充
        for i in 100..128 {
            features[i] = if n > 0.0 { features[(i % 100)] * 0.1 } else { 0.0 };
        }

        features
    }

    /// 记录行为事件
    pub fn record_event(&self, event: BehaviorEvent) -> iot_ztna_common::Result<()> {
        let mut histories = self.histories.write().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取行为历史写锁失败: {}", e))
        })?;

        let spiffe = event.spiffe_id.to_string();
        let history = histories.entry(spiffe.clone()).or_default();

        // 清理窗口外事件
        let cutoff = chrono::Utc::now() - chrono::Duration::seconds(self.window_size);
        history.retain(|e| e.timestamp > cutoff);
        history.push(event);

        Ok(())
    }

    /// 评估设备行为异常
    pub fn assess(&self, spiffe_id: &iot_ztna_common::SpiffeId) -> iot_ztna_common::Result<BehaviorAssessment> {
        let spiffe = spiffe_id.to_string();

        let histories = self.histories.read().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取行为历史读锁失败: {}", e))
        })?;

        let history = histories.get(&spiffe);

        if history.is_none() || history.unwrap().len() < self.min_samples {
            return Ok(BehaviorAssessment::default());
        }

        let events = history.unwrap();

        // 提取特征向量
        let features = Self::extract_features(events);

        // Autoencoder 重建误差
        let model = self.model.read().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取模型读锁失败: {}", e))
        })?;
        let recon_error = model.reconstruction_error(&features);

        // 校准映射
        let calibration = self.calibration.read().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取校准读锁失败: {}", e))
        })?;
        let anomaly_score = map_to_anomaly_score(
            recon_error,
            calibration.lower_anchor,
            calibration.upper_anchor,
        );

        // SHAP-style 特征贡献度分解
        let mut feature_contributions = Vec::new();
        let baselines = self.baselines.read().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取基线读锁失败: {}", e))
        })?;

        if let Some(baseline) = baselines.get(&spiffe) {
            // 频率贡献
            let current_rate = events.len() as f64 / (self.window_size as f64 / 60.0);
            let expected_rate = baseline.protocol_rates.get("modbus")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let freq_contribution = if expected_rate > 0.0 {
                let z = (current_rate - expected_rate).abs() / expected_rate.sqrt();
                (z / 10.0).min(0.35)
            } else { 0.0 };
            feature_contributions.push(FeatureContribution {
                feature: "通信频次".to_string(),
                contribution: freq_contribution,
                value: format!("{:.1} 次/分", current_rate),
                baseline_value: format!("{:.1} 次/分", expected_rate),
            });

            // 操作贡献
            let recent_ops: Vec<&str> = events.iter().rev().take(20).map(|e| e.operation.as_str()).collect();
            let unknown_ops: Vec<String> = recent_ops.iter()
                .filter(|op| !baseline.common_operations.contains(&op.to_string()))
                .map(|s| s.to_string())
                .collect();
            let ops_contribution = (unknown_ops.len() as f64 * 0.05).min(0.25);
            feature_contributions.push(FeatureContribution {
                feature: "操作类型".to_string(),
                contribution: ops_contribution,
                value: format!("{}/{} 未知", unknown_ops.len(), recent_ops.len()),
                baseline_value: baseline.common_operations.join(","),
            });

            // 目标贡献
            let recent_targets: Vec<&str> = events.iter().rev().take(20).map(|e| e.target.as_str()).collect();
            let unknown_targets: Vec<String> = recent_targets.iter()
                .filter(|t| !baseline.common_targets.contains(&t.to_string()))
                .map(|s| s.to_string())
                .collect();
            let target_contribution = (unknown_targets.len() as f64 * 0.05).min(0.25);
            feature_contributions.push(FeatureContribution {
                feature: "访问目标".to_string(),
                contribution: target_contribution,
                value: format!("{}/{} 未知", unknown_targets.len(), recent_targets.len()),
                baseline_value: baseline.common_targets.join(","),
            });

            // 数据大小贡献
            let recent_sizes: Vec<f64> = events.iter().rev().take(20)
                .filter_map(|e| e.data_size.map(|s| s as f64))
                .collect();
            let data_contribution = if !recent_sizes.is_empty() {
                let avg_recent = recent_sizes.iter().sum::<f64>() / recent_sizes.len() as f64;
                let ratio = if baseline.avg_data_size > 0.0 {
                    avg_recent / baseline.avg_data_size
                } else { 1.0 };
                if ratio > 3.0 || ratio < 0.33 { 0.15 } else { 0.0 }
            } else { 0.0 };
            feature_contributions.push(FeatureContribution {
                feature: "数据大小".to_string(),
                contribution: data_contribution,
                value: format!("{:.0} bytes", recent_sizes.iter().sum::<f64>() / recent_sizes.len().max(1) as f64),
                baseline_value: format!("{:.0} bytes", baseline.avg_data_size),
            });
        }

        // 重建误差贡献
        feature_contributions.insert(0, FeatureContribution {
            feature: "重建误差(AE)".to_string(),
            contribution: anomaly_score * 0.4,
            value: format!("{:.6}", recon_error),
            baseline_value: format!("{:.6}", calibration.lower_anchor),
        });

        let is_anomaly = anomaly_score >= self.anomaly_threshold;
        let recommended_action = if is_anomaly {
            PolicyDecision::Deny
        } else if anomaly_score > self.anomaly_threshold * 0.7 {
            PolicyDecision::Log
        } else {
            PolicyDecision::Allow
        };

        let reason = if is_anomaly {
            format!("AE 异常评分 {:.2}: 重建误差 {:.6}", anomaly_score, recon_error)
        } else if anomaly_score > 0.0 {
            format!("轻微异常评分 {:.2}", anomaly_score)
        } else {
            "正常行为".to_string()
        };

        Ok(BehaviorAssessment {
            anomaly_score,
            is_anomaly,
            anomaly_types: if is_anomaly {
                vec!["重建误差异常".to_string()]
            } else {
                vec![]
            },
            recommended_action,
            reason,
            feature_contributions,
        })
    }

    /// 更新设备行为基线
    pub fn update_baseline(&self, spiffe_id: &iot_ztna_common::SpiffeId) -> iot_ztna_common::Result<()> {
        let spiffe = spiffe_id.to_string();

        let histories = self.histories.read().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取行为历史读锁失败: {}", e))
        })?;

        let events = match histories.get(&spiffe) {
            Some(e) if e.len() >= self.min_samples => e,
            _ => return Ok(()),
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
            hours.push(event.timestamp.format("%H").to_string().parse().unwrap_or(0));
        }

        let window_minutes = self.window_size as f64 / 60.0;
        let protocol_rates = serde_json::to_value(
            protocol_counts.iter().map(|(k, v)| {
                (k.clone(), *v as f64 / window_minutes)
            }).collect::<HashMap<String, f64>>()
        ).unwrap_or(serde_json::Value::Null);

        let mut ops_vec: Vec<(String, u32)> = operations.into_iter().collect();
        ops_vec.sort_by(|a, b| b.1.cmp(&a.1));
        let common_operations = ops_vec.iter().take(5).map(|(k, _)| k.clone()).collect();

        let mut targets_vec: Vec<(String, u32)> = targets.into_iter().collect();
        targets_vec.sort_by(|a, b| b.1.cmp(&a.1));
        let common_targets = targets_vec.iter().take(5).map(|(k, _)| k.clone()).collect();

        let active_hours = if hours.is_empty() {
            (0, 24)
        } else {
            let min_hour = hours.iter().min().unwrap();
            let max_hour = hours.iter().max().unwrap();
            (*min_hour, (*max_hour + 1).min(24))
        };

        let avg_data_size = if data_count > 0 {
            total_data_size / data_count as f64
        } else { 0.0 };

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

        // 同时更新校准参数
        let features = Self::extract_features(events);
        let model = self.model.read().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取模型读锁失败: {}", e))
        })?;
        let error = model.reconstruction_error(&features);

        let mut calib = self.calibration.write().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取校准写锁失败: {}", e))
        })?;
        // 更新下锚为中位数（近似为当前误差的 50%）
        calib.lower_anchor = (calib.lower_anchor * 0.9 + error * 0.1).max(0.001);
        // 更新上锚为 99.5 百分位近似
        calib.upper_anchor = (calib.upper_anchor * 0.95 + error * 3.0 * 0.05).max(calib.lower_anchor * 2.0);
        calib.calibration_samples += events.len();
        calib.calibrated_at = Utc::now();

        let mut baselines = self.baselines.write().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取基线写锁失败: {}", e))
        })?;
        baselines.insert(spiffe, baseline);

        Ok(())
    }

    /// 获取设备行为基线
    pub fn get_baseline(&self, spiffe_id: &iot_ztna_common::SpiffeId) -> iot_ztna_common::Result<Option<BehaviorBaseline>> {
        let baselines = self.baselines.read().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取基线读锁失败: {}", e))
        })?;
        Ok(baselines.get(&spiffe_id.to_string()).cloned())
    }

    /// 热加载新模型
    pub fn hot_reload_model(&self, model: AutoencoderModel) -> iot_ztna_common::Result<()> {
        let mut current = self.model.write().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取模型写锁失败: {}", e))
        })?;
        *current = model;
        tracing::info!("Autoencoder 模型已热加载");
        Ok(())
    }

    /// 获取当前模型信息
    pub fn get_model_info(&self) -> iot_ztna_common::Result<ModelInfo> {
        let model = self.model.read().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取模型读锁失败: {}", e))
        })?;
        let calib = self.calibration.read().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取校准读锁失败: {}", e))
        })?;

        Ok(ModelInfo {
            version: model.version.clone(),
            input_dim: model.input_dim,
            hidden_dims: model.hidden_dims.clone(),
            latent_dim: model.latent_dim,
            epochs: model.epochs,
            created_at: model.created_at,
            calibration: calib.clone(),
        })
    }

    /// 获取行为统计
    pub fn get_stats(&self) -> iot_ztna_common::Result<BehaviorStats> {
        let histories = self.histories.read().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取行为历史读锁失败: {}", e))
        })?;
        let baselines = self.baselines.read().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取基线读锁失败: {}", e))
        })?;

        let total_events: usize = histories.values().map(|e| e.len()).sum();

        Ok(BehaviorStats {
            total_events,
            devices_with_history: histories.len(),
            devices_with_baseline: baselines.len(),
        })
    }

    /// 清理过期行为历史
    pub fn cleanup(&self) -> iot_ztna_common::Result<usize> {
        let cutoff = chrono::Utc::now() - chrono::Duration::seconds(self.window_size);
        let mut histories = self.histories.write().map_err(|e| {
            iot_ztna_common::Error::Internal(format!("获取行为历史写锁失败: {}", e))
        })?;

        let mut removed = 0;
        for events in histories.values_mut() {
            let before = events.len();
            events.retain(|e| e.timestamp > cutoff);
            removed += before - events.len();
        }
        histories.retain(|_, events| !events.is_empty());

        Ok(removed)
    }
}

impl Default for AutoencoderEngine {
    fn default() -> Self { Self::new() }
}

/// 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub version: String,
    pub input_dim: usize,
    pub hidden_dims: Vec<usize>,
    pub latent_dim: usize,
    pub epochs: usize,
    pub created_at: DateTime<Utc>,
    pub calibration: CalibrationParams,
}

/// 行为统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorStats {
    pub total_events: usize,
    pub devices_with_history: usize,
    pub devices_with_baseline: usize,
}

// === 辅助函数 ===

fn leaky_relu(x: f64) -> f64 {
    if x > 0.0 { x } else { 0.01 * x }
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// 重建误差 → 异常评分（线性映射）
fn map_to_anomaly_score(error: f64, lower: f64, upper: f64) -> f64 {
    if upper <= lower {
        return 0.0;
    }
    let score = (error - lower) / (upper - lower);
    score.clamp(0.0, 1.0)
}

/// 简单字符串哈希
fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}
