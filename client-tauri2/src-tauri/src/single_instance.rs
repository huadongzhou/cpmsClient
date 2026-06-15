//! 单实例保护：用回环端口作为进程锁，避免托盘/socket worker/日志文件被重复实例争用。
//! 纯 std::net 实现，不引入插件依赖（兼顾 legacy 壳的依赖链约束）。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use tauri::AppHandle;

const LOCK_ADDR: &str = "127.0.0.1:51987";
const MAGIC: &[u8] = b"CPMS_CLIENT_SINGLETON_V1";
const MAGIC_ACK: &[u8] = b"CPMS_CLIENT_SINGLETON_ACK";

pub enum Acquire {
    /// 本进程是首个实例，需持有该 listener 至进程结束。
    Primary(TcpListener),
    /// 已有本应用实例在运行（已请求其显示窗口），调用方应退出。
    Secondary,
    /// 锁端口被其他程序占用，无法保护，按无单实例放行。
    Foreign,
}

/// 尝试成为首个实例。
pub fn try_acquire() -> Acquire {
    match TcpListener::bind(LOCK_ADDR) {
        Ok(listener) => Acquire::Primary(listener),
        Err(_) => {
            if notify_existing() {
                Acquire::Secondary
            } else {
                Acquire::Foreign
            }
        }
    }
}

/// 首实例持有锁后调用：在后台监听二次启动的唤醒请求并显示主窗口。
pub fn serve(listener: TcpListener, app: AppHandle) {
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };

            let mut buf = [0u8; 64];
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
            if let Ok(read) = stream.read(&mut buf) {
                if buf[..read].starts_with(MAGIC) {
                    let _ = stream.write_all(MAGIC_ACK);
                    crate::services::log_service::info(
                        &app,
                        "single-instance",
                        "检测到二次启动，已唤醒并显示主窗口",
                    );
                    crate::window::show_main_window(&app);
                }
            }
        }
    });
}

/// 二次启动时连接首实例并握手；确认是本应用则返回 true。
fn notify_existing() -> bool {
    let Ok(addr) = LOCK_ADDR.parse() else {
        return false;
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(500)) else {
        return false;
    };

    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    if stream.write_all(MAGIC).is_err() {
        return false;
    }

    let mut buf = [0u8; 64];
    match stream.read(&mut buf) {
        Ok(read) => buf[..read].starts_with(MAGIC_ACK),
        Err(_) => false,
    }
}
