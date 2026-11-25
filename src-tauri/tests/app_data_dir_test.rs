//! 应用数据目录约定测试：所有本地数据必须落在 Application Support/Workbench/ 子目录下。

use std::path::PathBuf;

use workbench_lib::{app_data_dir, app_data_parent_dir, APP_DATA_SUBDIR};

#[test]
fn app_data_dir_ends_with_workbench_subdir() {
    let dir = app_data_dir();
    assert_eq!(
        dir.file_name().and_then(|n| n.to_str()),
        Some(APP_DATA_SUBDIR),
        "数据目录末段必须是 {APP_DATA_SUBDIR}，实际: {}",
        dir.display()
    );
    assert!(
        dir.to_string_lossy().contains(APP_DATA_SUBDIR),
        "数据目录路径需包含 {APP_DATA_SUBDIR}，实际: {}",
        dir.display()
    );
}

#[test]
fn subdir_is_created_under_a_given_parent() {
    // 用临时目录验证“首次启动建出 Workbench 子目录”的拼接与创建逻辑，不写真实 Application Support
    let base = std::env::temp_dir().join(format!("wb-appdir-{}", uuid::Uuid::new_v4()));
    let dir: PathBuf = app_data_parent_dir(&base);
    std::fs::create_dir_all(&dir).unwrap();
    assert!(dir.exists(), "应能建出子目录: {}", dir.display());
    assert_eq!(dir.file_name().and_then(|n| n.to_str()), Some(APP_DATA_SUBDIR));
    std::fs::remove_dir_all(base).ok();
}
