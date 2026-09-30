//! Modbus TCP 代理转发模块
//!
//! 实现 Modbus TCP 代理功能：
//! - 功能码过滤（允许/拒绝特定功能码）
//! - 寄存器地址范围限制
//! - TCP 透传 + 策略检查

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing;

use iot_ztna_policy::PolicyManager;

/// Modbus 代理配置
#[derive(Clone)]
pub struct ModbusProxyConfig {
    pub listen_addr: String,
    pub listen_port: u16,
    pub backend_host: String,
    pub backend_port: u16,
    pub max_connections: usize,
    pub allowed_function_codes: Vec<u8>,
    pub allowed_register_range: Option<(u16, u16)>,
    pub connection_timeout: u64,
}

impl Default for ModbusProxyConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".to_string(),
            listen_port: 5020,
            backend_host: "127.0.0.1".to_string(),
            backend_port: 502,
            max_connections: 100,
            allowed_function_codes: vec![1, 2, 3, 4, 5, 6, 15, 16],
            allowed_register_range: None,
            connection_timeout: 30,
        }
    }
}

/// Modbus 代理服务
pub struct ModbusProxy {
    config: ModbusProxyConfig,
    policy_manager: Arc<PolicyManager>,
}

impl ModbusProxy {
    pub fn new(config: ModbusProxyConfig, policy_manager: Arc<PolicyManager>) -> Self {
        Self { config, policy_manager }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let addr = format!("{}:{}", self.config.listen_addr, self.config.listen_port);
        let listener = TcpListener::bind(&addr).await?;
        tracing::info!(
            addr = %addr,
            backend = format!("{}:{}", self.config.backend_host, self.config.backend_port),
            "Modbus TCP 代理监听中"
        );

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    tracing::debug!(peer = %peer_addr, "Modbus 客户端连接");
                    let config = self.config.clone();
                    let pm = self.policy_manager.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_modbus_connection(stream, &config, &pm, peer_addr).await {
                            tracing::warn!(peer = %peer_addr, error = %e, "Modbus 连接处理错误");
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(error = %e, "Modbus 接受连接失败");
                }
            }
        }
    }
}

/// 处理单个 Modbus TCP 连接
async fn handle_modbus_connection(
    client_stream: tokio::net::TcpStream,
    config: &ModbusProxyConfig,
    _policy_manager: &PolicyManager,
    peer_addr: std::net::SocketAddr,
) -> anyhow::Result<()> {
    let backend_stream = tokio::net::TcpStream::connect(format!(
        "{}:{}", config.backend_host, config.backend_port
    ))
    .await?;

    // 使用 Arc<Mutex> 共享 client_stream 的写端
    let (mut client_rx, client_tx) = client_stream.into_split();
    let (mut backend_rx, mut backend_tx) = backend_stream.into_split();
    let client_tx = Arc::new(Mutex::new(client_tx));

    let allowed_fc = config.allowed_function_codes.clone();
    let allowed_range = config.allowed_register_range;

    // 客户端 → 后端（请求方向，检查功能码）
    let client_tx_c2b = client_tx.clone();
    let client_to_backend = async {
        let mut buf = vec![0u8; 4096];
        loop {
            match (&mut client_rx).read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if n >= 8 {
                        let function_code = buf[7];

                        // 检查功能码
                        if !allowed_fc.is_empty() && !allowed_fc.contains(&function_code) {
                            tracing::warn!(peer = %peer_addr, fc = function_code, "Modbus 功能码被拒绝");
                            let mut exception_response = buf[..n].to_vec();
                            exception_response[7] = function_code | 0x80;
                            exception_response[8] = 0x01;
                            let _ = client_tx_c2b.lock().await.write_all(&exception_response).await;
                            continue;
                        }

                        // 检查寄存器地址范围
                        if let Some((start, end)) = allowed_range {
                            if n >= 12 && matches!(function_code, 3 | 4 | 6 | 16) {
                                let reg_addr = u16::from_be_bytes([buf[8], buf[9]]);
                                if reg_addr < start || reg_addr > end {
                                    tracing::warn!(peer = %peer_addr, reg_addr, "Modbus 寄存器地址超出允许范围");
                                    let mut exception_response = buf[..n].to_vec();
                                    exception_response[7] = function_code | 0x80;
                                    exception_response[8] = 0x02;
                                    let _ = client_tx_c2b.lock().await.write_all(&exception_response).await;
                                    continue;
                                }
                            }
                        }

                        tracing::debug!(peer = %peer_addr, fc = function_code, size = n, "Modbus 请求通过");
                    }

                    if backend_tx.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    // 后端 → 客户端（响应方向，透传）
    let backend_to_client = async {
        let mut buf = vec![0u8; 4096];
        loop {
            match (&mut backend_rx).read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let _ = client_tx.lock().await.write_all(&buf[..n]).await;
                }
                Err(_) => break,
            }
        }
    };

    tokio::select! {
        _ = client_to_backend => {}
        _ = backend_to_client => {}
    }

    tracing::debug!(peer = %peer_addr, "Modbus 连接关闭");
    Ok(())
}
