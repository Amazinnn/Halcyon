//! focus-cli (v1.5): the local control-plane client. Reads the running Focus
//! Desktop's port + token from `%APPDATA%\com.focusdesktop.app\cli.json`,
//! sends one JSON command over localhost TCP, prints the JSON response.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;

const CLI_FILE: &str = "cli.json";
const APP_DIR: &str = "com.focusdesktop.app";
const FRAME_MAX: usize = 1 << 20;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        print_help();
        return;
    }
    let mut agent_thread: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--agent-thread" {
            if i + 1 < args.len() {
                agent_thread = Some(args.remove(i + 1));
                args.remove(i);
            } else {
                eprintln!("[focus-cli] --agent-thread 需要参数");
                std::process::exit(2);
            }
        } else {
            i += 1;
        }
    }
    let cmd = args.join(" ");

    let appdata = match std::env::var("APPDATA") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("[focus-cli] APPDATA not set");
            std::process::exit(2);
        }
    };
    let meta_path = PathBuf::from(appdata).join(APP_DIR).join(CLI_FILE);
    let meta: serde_json::Value = match std::fs::read_to_string(&meta_path) {
        Ok(t) => match serde_json::from_str(&t) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[focus-cli] 解析 {} 失败: {e}", meta_path.display());
                std::process::exit(2);
            }
        },
        Err(_) => {
            eprintln!("[focus-cli] Focus Desktop 未在运行（找不到 {}）", meta_path.display());
            std::process::exit(3);
        }
    };
    let port = meta["port"].as_u64().unwrap_or(0);
    let token = meta["token"].as_str().unwrap_or("").to_string();
    if port == 0 || token.is_empty() {
        eprintln!("[focus-cli] {} 内容无效", meta_path.display());
        std::process::exit(2);
    }

    let addr = format!("127.0.0.1:{port}");
    let mut stream = match TcpStream::connect(&addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[focus-cli] 无法连接 {addr}: {e}");
            std::process::exit(4);
        }
    };

    let mut req = serde_json::json!({ "token": token, "cmd": cmd });
    if let Some(tid) = agent_thread {
        req["agentThread"] = serde_json::json!(tid);
    }
    let payload = serde_json::to_vec(&req).unwrap_or_default();
    if stream.write_all(&(payload.len() as u32).to_le_bytes()).is_err()
        || stream.write_all(&payload).is_err()
    {
        eprintln!("[focus-cli] 写入失败");
        std::process::exit(5);
    }

    let mut len_buf = [0u8; 4];
    if stream.read_exact(&mut len_buf).is_err() {
        eprintln!("[focus-cli] 读取响应失败");
        std::process::exit(6);
    }
    let rlen = u32::from_le_bytes(len_buf) as usize;
    if rlen == 0 || rlen > FRAME_MAX {
        eprintln!("[focus-cli] 响应长度异常");
        std::process::exit(6);
    }
    let mut buf = vec![0u8; rlen];
    if stream.read_exact(&mut buf).is_err() {
        eprintln!("[focus-cli] 读取响应失败");
        std::process::exit(6);
    }
    match serde_json::from_slice::<serde_json::Value>(&buf) {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default()),
        Err(_) => println!("{}", String::from_utf8_lossy(&buf)),
    }
}

fn print_help() {
    println!("focus-cli — Focus Desktop 本地控制面（需先运行 Focus Desktop）");
    println!("用法：");
    println!("  focus-cli timer start|pause|skip|status");
    println!("  focus-cli stats today|week|sessions");
    println!("  focus-cli desktop layout");
    println!("  focus-cli apps now|visible");
    println!("  focus-cli ping");
    println!("  focus-cli --agent-thread <thread_id> <command>  （Agent 专用：白名单+审计）");
}
