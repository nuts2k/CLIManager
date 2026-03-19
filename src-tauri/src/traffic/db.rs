use rusqlite::Connection;
use std::path::PathBuf;

use crate::traffic::schema::MIGRATIONS;

/// 获取 traffic.db 的标准路径（非 iCloud）
///
/// macOS: ~/Library/Application Support/com.climanager.app/traffic.db
/// 使用 dirs::data_local_dir() 而非 Tauri PathResolver，保持与现有 storage 模块风格一致，
/// 且不需要 AppHandle 引用，可在任意上下文调用。
pub fn get_traffic_db_path() -> PathBuf {
    let base = dirs::data_local_dir().expect("无法获取本地数据目录（data_local_dir 返回 None）");
    // Tauri bundle identifier: com.climanager.app（来自 tauri.conf.json）
    base.join("com.climanager.app").join("traffic.db")
}

/// 配置 SQLite 连接级 PRAGMA
///
/// - journal_mode=WAL：写时复制，读写并发性更好；持久化在 DB 文件，只需设一次，
///   但 configure_connection 每次打开都调用以确保 busy_timeout 生效
/// - busy_timeout=5000：连接级参数（非持久化），等待锁最多 5s，防止 SQLITE_BUSY
/// - synchronous=NORMAL：WAL 模式下比 FULL 更快且仍安全
/// - foreign_keys=ON：启用外键约束（当前表设计无 FK，但作为防御性配置）
fn configure_connection(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;
        PRAGMA busy_timeout=5000;
        PRAGMA synchronous=NORMAL;
        PRAGMA foreign_keys=ON;
        ",
    )
}

/// 尝试打开并迁移 DB
///
/// 步骤：create_dir_all → Connection::open → configure_connection → MIGRATIONS.to_latest
fn try_open_and_migrate(path: &std::path::Path) -> Result<Connection, Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    configure_connection(&conn)?;
    MIGRATIONS.to_latest(&mut conn)?;
    Ok(conn)
}

/// 打开 traffic.db，失败时自动删除并重建（降级运行策略）
///
/// 返回值：
/// - Some(Connection)：成功打开并完成 schema 迁移
/// - None：重建也失败（磁盘写保护、权限不足等），应用降级运行，代理正常但不记录流量
pub fn open_traffic_db() -> Option<Connection> {
    let path = get_traffic_db_path();
    match try_open_and_migrate(&path) {
        Ok(conn) => Some(conn),
        Err(e) => {
            log::warn!(
                "traffic.db 初始化失败，尝试删除重建: {} (path: {:?})",
                e,
                path
            );
            let _ = std::fs::remove_file(&path);
            match try_open_and_migrate(&path) {
                Ok(conn) => {
                    log::info!("traffic.db 重建成功");
                    Some(conn)
                }
                Err(e2) => {
                    log::error!(
                        "traffic.db 重建失败，降级运行（代理正常工作，流量不记录）: {} (path: {:?})",
                        e2,
                        path
                    );
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// DB 路径包含 "Application Support"，不包含 "Mobile Documents"（非 iCloud）
    #[test]
    fn test_db_path_not_icloud() {
        let path = get_traffic_db_path();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("Application Support"),
            "路径应包含 'Application Support'，实际路径: {path_str}"
        );
        assert!(
            !path_str.contains("Mobile Documents"),
            "路径不应包含 'Mobile Documents'（iCloud 路径），实际路径: {path_str}"
        );
        assert!(
            !path_str.contains("CloudDocs"),
            "路径不应包含 'CloudDocs'（iCloud 路径），实际路径: {path_str}"
        );
        assert!(
            path_str.ends_with("traffic.db"),
            "路径应以 traffic.db 结尾，实际路径: {path_str}"
        );
    }

    /// WAL 模式已配置（journal_mode=wal）
    #[test]
    fn test_wal_mode() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut conn = Connection::open(&db_path).unwrap();
        configure_connection(&conn).unwrap();
        MIGRATIONS.to_latest(&mut conn).unwrap();

        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            mode.to_lowercase(),
            "wal",
            "journal_mode 应为 wal，实际: {mode}"
        );
    }

    /// busy_timeout 已配置（5000ms）
    #[test]
    fn test_busy_timeout() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut conn = Connection::open(&db_path).unwrap();
        configure_connection(&conn).unwrap();
        MIGRATIONS.to_latest(&mut conn).unwrap();

        let timeout: i64 = conn
            .query_row("PRAGMA busy_timeout", [], |r| r.get(0))
            .unwrap();
        assert_eq!(timeout, 5000, "busy_timeout 应为 5000ms，实际: {timeout}ms");
    }

    /// 首次打开 DB 成功，返回可用连接
    #[test]
    fn test_open_db_success() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("traffic.db");
        let conn = try_open_and_migrate(&db_path);
        assert!(conn.is_ok(), "首次打开 DB 应成功，错误: {:?}", conn.err());

        // 验证表已创建
        let conn = conn.unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='request_logs'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "request_logs 表应已创建");
    }

    /// DB 文件损坏时 open_traffic_db 能自动重建（通过写入无效内容模拟损坏）
    #[test]
    fn test_corrupted_db_recovery() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("com.climanager.app").join("traffic.db");
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();

        // 写入损坏内容
        std::fs::write(&db_path, b"this is not a valid sqlite database file").unwrap();

        // try_open_and_migrate 应失败
        let result = try_open_and_migrate(&db_path);
        assert!(result.is_err(), "损坏的 DB 打开应失败");

        // 删除后重建应成功
        let _ = std::fs::remove_file(&db_path);
        let result = try_open_and_migrate(&db_path);
        assert!(result.is_ok(), "重建 DB 应成功，错误: {:?}", result.err());
    }
}
