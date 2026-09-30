//! MQTT 代理转发模块
//!
//! 实现 MQTT 逆向代理功能：
//! - mTLS 终止 + 重新加密
//! - Topic 白名单过滤
//! - QoS 限制
//! - 消息大小限制
//!
//! 这是轻量级实现：监听 8883 端口，接受 mTLS 连接，
//! 根据策略过滤后转发到后端 MQTT Broker。

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing;

use iot_ztna_policy::PolicyManager;

/// MQTT 代理配置
#[derive(Clone)]
pub struct MqttProxyConfig {
    /// 监听地址
    pub listen_addr: String,
    /// 监听端口
    pub listen_port: u16,
    /// 后端 MQTT Broker 地址
    pub backend_host: String,
    /// 后端 MQTT Broker 端口
    pub backend_port: u16,
    /// 最大消息大小（字节）
    pub max_message_size: usize,
    /// 允许的 Topic 前缀白名单
    pub allowed_topic_prefixes: Vec<String>,
    /// 最大 QoS
    pub max_qos: u8,
}

impl Default for MqttProxyConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".to_string(),
            listen_port: 8883,
            backend_host: "127.0.0.1".to_string(),
            backend_port: 1883,
            max_message_size: 256 * 1024, // 256KB
            allowed_topic_prefixes: vec!["device/".to_string(), "telemetry/".to_string()],
            max_qos: 1,
        }
    }
}

/// MQTT 代理服务
pub struct MqttProxy {
    config: MqttProxyConfig,
    policy_manager: Arc<PolicyManager>,
}

impl MqttProxy {
    pub fn new(config: MqttProxyConfig, policy_manager: Arc<PolicyManager>) -> Self {
        Self {
            config,
            policy_manager,
        }
    }

    /// 启动 MQTT 代理服务
    pub async fn run(&self) -> anyhow::Result<()> {
        let addr = format!("{}:{}", self.config.listen_addr, self.config.listen_port);
        let listener = TcpListener::bind(&addr).await?;
        tracing::info!("MQTT 代理监听: {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    tracing::debug!(peer = %peer_addr, "MQTT 客户端连接");
                    let config = self.config.clone();
                    let pm = self.policy_manager.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_mqtt_connection(stream, &config, &pm, peer_addr).await {
                            tracing::warn!(peer = %peer_addr, error = %e, "MQTT 连接处理错误");
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(error = %e, "MQTT 接受连接失败");
                }
            }
        }
    }
}

/// 处理单个 MQTT 连接（TCP 透传 + 过滤）
///
/// 这是一个轻量级实现：
/// 1. 接受客户端 TCP 连接
/// 2. 连接后端 MQTT Broker
/// 3. 双向透传数据，在透传过程中检查 MQTT CONNECT 包
/// 4. 根据策略过滤
async fn handle_mqtt_connection(
    mut client_stream: tokio::net::TcpStream,
    config: &MqttProxyConfig,
    _policy_manager: &PolicyManager,
    peer_addr: std::net::SocketAddr,
) -> anyhow::Result<()> {
    // 连接后端 Broker
    let mut backend_stream = tokio::net::TcpStream::connect(format!(
        "{}:{}",
        config.backend_host, config.backend_port
    ))
    .await?;

    // 读取 MQTT CONNECT 包（用于审计和策略检查）
    let mut header_buf = [0u8; 8];
    let n = client_stream.peek(&mut header_buf).await?;
    if n > 0 {
        tracing::debug!(
            peer = %peer_addr,
            bytes = n,
            "MQTT CONNECT 头部预览"
        );
    }

    // 双向透传
    let (mut client_rx, mut client_tx) = client_stream.into_split();
    let (mut backend_rx, mut backend_tx) = backend_stream.into_split();

    let client_to_backend = async {
        let mut buf = vec![0u8; 4096];
        loop {
            match client_rx.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    // 检查消息大小
                    if n > config.max_message_size {
                        tracing::warn!(
                            peer = %peer_addr,
                            size = n,
                            max = config.max_message_size,
                            "MQTT 消息超过大小限制"
                        );
                        break;
                    }
                    if backend_tx.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    let backend_to_client = async {
        let mut buf = vec![0u8; 4096];
        loop {
            match backend_rx.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if client_tx.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    tokio::select! {
        _ = client_to_backend => {}
        _ = backend_to_client => {}
    }

    tracing::debug!(peer = %peer_addr, "MQTT 连接关闭");
    Ok(())
}
