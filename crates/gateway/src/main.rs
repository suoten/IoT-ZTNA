//! IoT-ZTNA 网关主程序
//!
//! 零信任网络访问网关 - 边缘端核心入口
//! 负责初始化所有组件并启动 HTTP 服务

mod mqtt_proxy;
mod modbus_proxy;

use clap::{Arg, Command};
use iot_ztna_api::{auth::AuthConfig, create_router, AppState};
use iot_ztna_common::{BehaviorEngine, GatewayConfig};
use iot_ztna_identity::{
    ca::CertificateAuthority, registration::RegistrationService,
    revocation::RevocationService, svid::SvidManager,
};
use iot_ztna_policy::{executor::NftablesExecutor, manager::PolicyManager, PolicyExecutor};
use iot_ztna_storage::Database;
use iot_ztna_dataplane::DataPlaneManager;
use iot_ztna_ai::{AutoencoderEngine, ModelManager};
use iot_ztna_opa::{RegoEngine, BundleManager};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // CLI 参数解析
    let matches = Command::new("IoT-ZTNA Gateway")
        .version("1.0.0")
        .about("零信任网络访问网关 - IoT 设备身份与策略前置层")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("配置文件路径 (TOML)")
                .default_value("config/ztna.toml"),
        )
        .arg(
            Arg::new("init-ca")
                .long("init-ca")
                .action(clap::ArgAction::SetTrue)
                .help("初始化 Root CA 并退出"),
        )
        .arg(
            Arg::new("init-policy")
                .long("init-policy")
                .action(clap::ArgAction::SetTrue)
                .help("初始化默认策略并退出"),
        )
        .arg(
            Arg::new("train-model")
                .long("train-model")
                .action(clap::ArgAction::SetTrue)
                .help("训练 Autoencoder 模型并退出"),
        )
        .get_matches();

    let config_path = matches.get_one::<String>("config").map(PathBuf::from).unwrap();

    // 加载配置
    let config = if config_path.exists() {
        tracing::info!("从 {} 加载配置", config_path.display());
        GatewayConfig::from_file(&config_path)?
    } else {
        tracing::warn!("配置文件 {} 不存在，使用默认配置", config_path.display());
        let config = GatewayConfig::default();
        // 保存默认配置
        if let Err(e) = config.save_to_file(&config_path) {
            tracing::warn!("保存默认配置失败: {}", e);
        }
        config
    };

    // 初始化日志
    let log_level = config.log_level.clone();
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&log_level));
    fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!(
        gateway_id = %config.gateway_id,
        trust_domain = %config.trust_domain,
        listen_addr = %config.listen_addr,
        listen_port = config.listen_port,
        "IoT-ZTNA 网关启动中..."
    );

    // 初始化数据库
    let db_path = PathBuf::from(&config.db_path);
    let db = Database::open(&db_path)?;
    tracing::info!(db_path = %db_path.display(), "数据库已初始化");

    // 初始化默认管理员用户
    iot_ztna_api::routes::user::init_default_users(&db)?;

    // 初始化 CA
    let ca_dir = PathBuf::from("data/certs");
    let ca = CertificateAuthority::load_or_generate(&ca_dir, &config.trust_domain)?;
    tracing::info!("证书签发机构 (CA) 已就绪");

    // 初始化 SVID 管理器
    let svid_manager = Arc::new(SvidManager::new(
        db.clone(),
        ca.clone(),
        config.svid_ttl_days,
        config.renewal_trigger_hours,
    ));

    // 初始化注册服务
    let registration = Arc::new(RegistrationService::new(
        db.clone(),
        &config.trust_domain,
    ));

    // 初始化吊销服务
    let revocation = Arc::new(RevocationService::new(db.clone()));

    // 初始化策略执行器（兼容旧版）
    let executor = Arc::new(NftablesExecutor::new(&config.nft_table));
    executor.init_table()?;

    // 初始化数据面管理器（eBPF + nftables 自动选择）
    let dataplane = Arc::new(DataPlaneManager::new(&config.nft_table));
    tracing::info!(
        mode = dataplane.mode(),
        is_ebpf = dataplane.is_ebpf(),
        "数据面已初始化"
    );

    // 初始化策略管理器
    let policy_manager = Arc::new(PolicyManager::new(
        db.clone(),
        executor.clone() as Arc<dyn PolicyExecutor>,
    ));
    policy_manager.init()?;
    tracing::info!("策略引擎已加载");

    // 初始化 Autoencoder 引擎
    let ae_engine = Arc::new(AutoencoderEngine::new());
    tracing::info!("Autoencoder 行为基线引擎已初始化");

    // 初始化模型管理器
    let model_manager = Arc::new(ModelManager::new(ae_engine.clone(), "data/models"));
    tracing::info!("ONNX 模型管理器已初始化");

    // 初始化 Rego 引擎
    let rego_engine = Arc::new(RegoEngine::new());
    // 加载默认 Rego 规则
    let default_rego_rules = iot_ztna_opa::RegoEngine::default_rego_rules();
    rego_engine.load_rules(default_rego_rules)?;
    tracing::info!("OPA/Rego 策略引擎已初始化");

    // 初始化 Bundle 管理器
    let bundle_manager = Arc::new(BundleManager::new(rego_engine.clone(), "data/bundles"));
    tracing::info!("OPA Bundle 管理器已初始化");

    // 如果是初始化模式，创建默认策略后退出
    if matches.get_flag("init-ca") {
        tracing::info!("Root CA 初始化完成，退出");
        return Ok(());
    }

    if matches.get_flag("init-policy") {
        let default_rules = iot_ztna_policy::engine::PolicyEngine::default_rules();
        let policy_id = policy_manager.create_policy(
            "默认策略",
            default_rules,
            "system",
        )?;
        policy_manager.publish_policy(&policy_id)?;
        tracing::info!(policy_id = %policy_id, "默认策略已创建并发布");
        return Ok(());
    }

    if matches.get_flag("train-model") {
        tracing::info!("开始训练 Autoencoder 模型...");
        let trainer = iot_ztna_ai::ModelTrainer::new(ae_engine.clone());
        let config = iot_ztna_ai::training::TrainingConfig::default();

        // 生成模拟训练数据
        let training_data = generate_training_data();
        let result = trainer.train(&training_data, &config)?;

        // 注册模型版本
        let version_id = model_manager.register_version(
            "1.0.0",
            result.epochs,
            result.training_samples,
            result.validation_loss,
            "system",
            "初始训练模型",
            result.calibration,
        )?;
        model_manager.activate_version(&version_id)?;

        tracing::info!(
            version_id = %version_id,
            final_loss = format!("{:.6}", result.final_loss),
            "模型训练完成并激活"
        );
        return Ok(());
    }

    // 创建广播通道
    let (alert_tx, _) = broadcast::channel::<String>(100);
    let (device_tx, _) = broadcast::channel::<String>(100);
    let (metrics_tx, _) = broadcast::channel::<String>(100);

    // 初始化 JWT 认证
    let jwt_secret = std::env::var("ZTNA_JWT_SECRET").unwrap_or_else(|_| {
        tracing::warn!("ZTNA_JWT_SECRET 未设置，使用默认密钥（仅限开发环境）");
        "iot-ztna-dev-secret-change-in-production".to_string()
    });
    let auth_enabled = std::env::var("ZTNA_AUTH_ENABLED")
        .map(|v| v != "false" && v != "0")
        .unwrap_or(true);
    let auth_config = Arc::new(AuthConfig::new(&jwt_secret, auth_enabled));
    tracing::info!(auth_enabled, "JWT 认证已初始化");

    // 创建应用状态
    let state = AppState {
        config: Arc::new(config.clone()),
        db: db.clone(),
        svid_manager,
        registration,
        revocation,
        policy_manager,
        alert_tx,
        device_tx,
        metrics_tx: metrics_tx.clone(),
        behavior_engine: Arc::new(BehaviorEngine::new()),
        ae_engine,
        model_manager,
        dataplane,
        rego_engine,
        bundle_manager,
        auth_config,
    };

    // 启动后台任务
    let bg_state = state.clone();
    tokio::spawn(async move {
        background_tasks(bg_state).await;
    });

    // 启动 MQTT 代理（后台运行）
    let mqtt_config = mqtt_proxy::MqttProxyConfig::default();
    let mqtt_proxy = mqtt_proxy::MqttProxy::new(mqtt_config, state.policy_manager.clone());
    tokio::spawn(async move {
        if let Err(e) = mqtt_proxy.run().await {
            tracing::error!(error = %e, "MQTT 代理服务错误");
        }
    });

    // 启动 Modbus TCP 代理（后台运行）
    let modbus_config = modbus_proxy::ModbusProxyConfig::default();
    let modbus_proxy = modbus_proxy::ModbusProxy::new(modbus_config, state.policy_manager.clone());
    tokio::spawn(async move {
        if let Err(e) = modbus_proxy.run().await {
            tracing::error!(error = %e, "Modbus TCP 代理服务错误");
        }
    });

    // 启动 HTTP 服务
    let router = create_router(state.clone());
    let addr = format!("{}:{}", config.listen_addr, config.listen_port);
    tracing::info!("HTTP 服务监听: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("HTTP 服务监听: http://{}", addr);

    // Graceful shutdown: 捕获 Ctrl+C / SIGTERM
    let shutdown = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        tracing::info!("收到关闭信号，开始优雅停机...");
    };

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|e| anyhow::anyhow!("HTTP 服务错误: {}", e))?;

    tracing::info!("网关已停止");
    Ok(())
}

/// 生成训练数据（用于 --train-model 命令）
fn generate_training_data() -> Vec<iot_ztna_common::BehaviorEvent> {
    use chrono::Utc;
    use iot_ztna_common::{BehaviorEvent, SpiffeId};

    let mut events = Vec::new();
    let spiffe = SpiffeId::parse("spiffe://iot-ztna.local/device/sensor-001").unwrap();

    for i in 0..500 {
        let event = BehaviorEvent {
            spiffe_id: spiffe.clone(),
            protocol: if i % 3 == 0 { "modbus".to_string() } else { "mqtt".to_string() },
            operation: if i % 5 == 0 { "write".to_string() } else { "read".to_string() },
            target: format!("10.0.0.{}:502", (i % 10) + 1),
            function_code: Some(((i % 4) + 1) as u8),
            data_size: Some(256 + (i % 100) * 10),
            timestamp: Utc::now() - chrono::Duration::seconds(i as i64),
        };
        events.push(event);
    }

    events
}

/// 后台任务：证书自动续期 + 过期清理 + 指标推送 + 行为清理
async fn background_tasks(state: AppState) {
    let renewal_interval = std::time::Duration::from_secs(3600); // 1 小时
    let cleanup_interval = std::time::Duration::from_secs(300); // 5 分钟
    let metrics_interval = std::time::Duration::from_secs(5);

    let mut renewal_timer = tokio::time::interval(renewal_interval);
    let mut cleanup_timer = tokio::time::interval(cleanup_interval);
    let mut metrics_timer = tokio::time::interval(metrics_interval);

    // 跳过第一次立即触发
    renewal_timer.tick().await;
    cleanup_timer.tick().await;
    metrics_timer.tick().await;

    loop {
        tokio::select! {
            _ = renewal_timer.tick() => {
                tracing::debug!("执行证书自动续期检查...");
                match state.svid_manager.auto_renew() {
                    Ok(renewed) => {
                        if !renewed.is_empty() {
                            tracing::info!(count = renewed.len(), "证书自动续期完成");
                            for svid in &renewed {
                                let _ = state.alert_tx.send(serde_json::json!({
                                    "event": "certificate_renewed",
                                    "spiffe_id": svid.spiffe_id.to_string(),
                                    "expires_at": svid.expires_at.to_rfc3339(),
                                }).to_string());
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "证书续期任务失败");
                    }
                }
            }
            _ = cleanup_timer.tick() => {
                match state.svid_manager.cleanup_expired() {
                    Ok(count) => {
                        if count > 0 {
                            tracing::info!(count = count, "清理过期证书");
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "清理过期证书失败");
                    }
                }
            }
            _ = metrics_timer.tick() => {
                let online = state.db.count_online_devices().unwrap_or(0);
                let total = state.db.count_devices().unwrap_or(0);
                let alerts = state.db.count_unacknowledged_alerts().unwrap_or(0);
                let metrics_msg = serde_json::json!({
                    "devices_total": total,
                    "devices_online": online,
                    "alerts_unack": alerts,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });
                let _ = state.metrics_tx.send(metrics_msg.to_string());
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(600)) => {
                // 行为历史清理（每 10 分钟）
                match state.behavior_engine.cleanup() {
                    Ok(n) if n > 0 => {
                        tracing::debug!(removed = n, "清理过期行为历史");
                    }
                    _ => {}
                }
                // Autoencoder 行为历史清理
                match state.ae_engine.cleanup() {
                    Ok(n) if n > 0 => {
                        tracing::debug!(removed = n, "清理过期 AE 行为历史");
                    }
                    _ => {}
                }
            }
        }
    }
}
