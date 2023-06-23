use super::config::Config;
use crate::utils::error::{PikaProxyError, Result};
use core::str::FromStr;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufStream, BufWriter},
    net::{TcpListener, TcpStream},
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

#[derive(Debug)]
pub struct Proxy {
    proxy: Arc<RwLock<_Proxy>>,
}

#[derive(Debug)]
struct _Proxy {
    xauth: String,

    online: bool,
    closed: bool,

    lproxy_addr: SocketAddr,
    ladmin_addr: SocketAddr,

    config: Config,
}

pub(crate) struct ProxyOptions {
    pub(crate) config_path: String,
}

impl From<&ProxyOptions> for Proxy {
    fn from(option: &ProxyOptions) -> Self {
        let config = Config::from_path(&option.config_path);
        let proxy = _Proxy {
            xauth: String::new(),

            online: false,
            closed: false,

            lproxy_addr: SocketAddr::from_str(&config.proxy_addr()).unwrap(),
            ladmin_addr: SocketAddr::from_str(&config.admin_addr()).unwrap(),

            config,
        };
        Proxy {
            proxy: Arc::new(RwLock::new(proxy)),
        }
    }
}

impl Proxy {
    async fn is_online(&self) -> bool {
        let proxy = self.r_lock().await;
        proxy.online && !proxy.closed
    }

    async fn is_closed(&self) -> bool {
        let proxy = self.r_lock().await;
        proxy.closed
    }

    async fn close(&self) {
        let mut proxy = self.w_lock().await;
        proxy.closed = true;
    }

    async fn r_lock(&self) -> RwLockReadGuard<'_, _Proxy> {
        self.proxy.read().await
    }

    async fn w_lock(&self) -> RwLockWriteGuard<'_, _Proxy> {
        self.proxy.write().await
    }

    async fn serve_admin(&self) {}

    pub async fn serve_proxy(&self) {
        if self.is_closed().await {
            return;
        }

        // 这里需要一条启动 log
        let proxy = self.proxy.read().await;
        println!("listen will on {:?}", proxy.lproxy_addr);
        // 挂起监听服务
        listen(&proxy.lproxy_addr).await;
        self.close().await
    }
}

//实际的挂起函数
async fn listen(addr: &SocketAddr) {
    let listener = TcpListener::bind(addr).await.unwrap();
    while let Ok((mut stream, _addr)) = listener.accept().await {
        tokio::spawn(do_task(stream));
    }
}

// 简单的打印服务器
async fn do_task(mut stream: TcpStream) {
    let mut buf_stream = BufStream::new(stream);
    let mut msg = vec![0; 1024];
    loop {
        match buf_stream.read(&mut msg).await {
            Ok(n) if n == 0 => continue,
            Ok(n) => {
                println!("{:?}", String::from_utf8((&msg[..n]).to_vec()));
                let size = buf_stream.write("+OK\r\n".as_bytes()).await.unwrap();
                println!("write_size: {}", size);
            }
            Err(e) => break,
        }
    }
}
