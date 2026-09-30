//! 模型训练器
//!
//! 功能:
//! - 离线训练 Autoencoder 模型
//! - 在线微调（增量训练）
//! - 校准参数自动计算
//! - 训练数据收集与预处理
//!
//! 训练流程:
//! 1. 收集设备正常行为数据（至少 7 天）
//! 2. 划分训练集（90%）和校准集（10%）
//! 3. 训练 Autoencoder（SGD + MSE Loss）
//! 4. 在校准集上计算重建误差分布
//! 5. 中位数 → 下锚（异常评分 = 0）
//! 6. 99.5 百分位 → 上锚（异常评分 = 1）

use crate::autoencoder::{AutoencoderEngine, AutoencoderModel, CalibrationParams};
use chrono::Utc;
use iot_ztna_common::{BehaviorEvent, Error, Result};
use std::sync::Arc;

/// 训练配置
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    pub epochs: usize,
    pub learning_rate: f64,
    pub batch_size: usize,
    pub train_ratio: f64,
    pub lower_percentile: f64,
    pub upper_percentile: f64,
    pub l2_reg: f64,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            epochs: 100,
            learning_rate: 0.001,
            batch_size: 32,
            train_ratio: 0.9,
            lower_percentile: 0.5,
            upper_percentile: 0.995,
            l2_reg: 0.0001,
        }
    }
}

/// 训练结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrainingResult {
    pub epochs: usize,
    pub final_loss: f64,
    pub validation_loss: f64,
    pub training_samples: usize,
    pub calibration_samples: usize,
    pub calibration: CalibrationParams,
    pub duration_secs: f64,
}

/// 模型训练器
pub struct ModelTrainer {
    engine: Arc<AutoencoderEngine>,
}

impl ModelTrainer {
    pub fn new(engine: Arc<AutoencoderEngine>) -> Self {
        Self { engine }
    }

    /// 训练 Autoencoder 模型
    pub fn train(
        &self,
        events: &[BehaviorEvent],
        config: &TrainingConfig,
    ) -> Result<TrainingResult> {
        let start = std::time::Instant::now();

        if events.len() < 100 {
            return Err(Error::Internal(format!(
                "训练数据不足: {} 条，至少需要 100 条",
                events.len()
            )));
        }

        let split_idx = (events.len() as f64 * config.train_ratio) as usize;
        let (train_data, calib_data) = events.split_at(split_idx);

        tracing::info!(
            total = events.len(),
            train = train_data.len(),
            calibrate = calib_data.len(),
            "训练数据准备完成"
        );

        let train_features: Vec<Vec<f64>> = train_data
            .chunks(config.batch_size)
            .map(|chunk| AutoencoderEngine::extract_features(chunk))
            .collect();

        let calib_features: Vec<Vec<f64>> = calib_data
            .chunks(config.batch_size)
            .map(|chunk| AutoencoderEngine::extract_features(chunk))
            .collect();

        let mut model = AutoencoderModel::new_default();

        let mut last_loss = f64::MAX;
        for epoch in 0..config.epochs {
            let mut epoch_loss = 0.0;

            for features in &train_features {
                let (reconstructed, _) = model.forward(features);
                let loss: f64 = features
                    .iter()
                    .zip(reconstructed.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    / features.len() as f64;

                epoch_loss += loss;
                self.backward_update(
                    &mut model,
                    features,
                    &reconstructed,
                    config.learning_rate,
                    config.l2_reg,
                );
            }

            epoch_loss /= train_features.len() as f64;
            last_loss = epoch_loss;

            if epoch % 10 == 0 || epoch == config.epochs - 1 {
                tracing::info!(
                    epoch = epoch,
                    loss = format!("{:.6}", epoch_loss),
                    "训练中"
                );
            }
        }

        let calib_errors: Vec<f64> = calib_features
            .iter()
            .map(|f| model.reconstruction_error(f))
            .collect();

        let calibration = self.calculate_calibration(
            &calib_errors,
            config.lower_percentile,
            config.upper_percentile,
        );

        self.engine.hot_reload_model(model)?;

        let duration = start.elapsed().as_secs_f64();

        let result = TrainingResult {
            epochs: config.epochs,
            final_loss: last_loss,
            validation_loss: calib_errors.iter().sum::<f64>() / calib_errors.len() as f64,
            training_samples: train_data.len(),
            calibration_samples: calib_data.len(),
            calibration,
            duration_secs: duration,
        };

        tracing::info!(
            final_loss = format!("{:.6}", result.final_loss),
            val_loss = format!("{:.6}", result.validation_loss),
            duration = format!("{:.1}s", duration),
            "训练完成"
        );

        Ok(result)
    }

    /// 在线微调（增量训练）
    pub fn fine_tune(
        &self,
        events: &[BehaviorEvent],
        config: &TrainingConfig,
    ) -> Result<TrainingResult> {
        let fine_tune_config = TrainingConfig {
            epochs: (config.epochs / 5).max(10),
            learning_rate: config.learning_rate * 0.1,
            ..config.clone()
        };

        tracing::info!("开始在线微调 ({} 轮)", fine_tune_config.epochs);
        self.train(events, &fine_tune_config)
    }

    fn calculate_calibration(
        &self,
        errors: &[f64],
        lower_pct: f64,
        upper_pct: f64,
    ) -> CalibrationParams {
        if errors.is_empty() {
            return CalibrationParams::default();
        }

        let mut sorted = errors.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let lower_idx = ((sorted.len() as f64 * lower_pct) as usize).min(sorted.len() - 1);
        let upper_idx = ((sorted.len() as f64 * upper_pct) as usize).min(sorted.len() - 1);

        let lower = sorted[lower_idx];
        let upper = sorted[upper_idx].max(lower * 2.0);

        CalibrationParams {
            lower_anchor: lower.max(0.001),
            upper_anchor: upper,
            calibration_samples: errors.len(),
            calibrated_at: Utc::now(),
        }
    }

    /// 简化的反向传播权重更新
    fn backward_update(
        &self,
        model: &mut AutoencoderModel,
        input: &[f64],
        reconstructed: &[f64],
        lr: f64,
        l2_reg: f64,
    ) {
        let error_grad: Vec<f64> = input
            .iter()
            .zip(reconstructed.iter())
            .map(|(a, b)| 2.0 * (b - a) / input.len() as f64)
            .collect();

        // 先获取潜在空间向量（避免借用冲突）
        let latent = model.encode(input);
        let latent_dim = latent.len();

        // 对解码器最后一层权重做梯度下降
        if let Some(last_weights) = model.decoder_weights.last_mut() {
            if let Some(last_biases) = model.decoder_biases.last_mut() {
                for (j, w_row) in last_weights.iter_mut().enumerate() {
                    let grad = error_grad.get(j).copied().unwrap_or(0.0);
                    for (k, w) in w_row.iter_mut().enumerate() {
                        let g = grad * latent.get(k % latent_dim).copied().unwrap_or(0.0)
                            + l2_reg * *w;
                        *w -= lr * g;
                    }
                    last_biases[j] -= lr * grad;
                }
            }
        }

        model.epochs += 1;
    }
}
