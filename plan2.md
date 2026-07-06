要将 dra 从单纯的"下载器"升级为具备**生命周期管理**（安装、记录、更新、卸载）的轻量级包管理器，核心在于引入**状态管理**和**标准化安装/卸载分发逻辑**。
以下是修改与整合计划。
---
### 一、 整体架构演进
dra 需要在现有的 GitHub API 和下载模块之上，新增一个**应用注册表** 和一个**生命周期管理器**。
```mermaid
flowchart TD
    A["dra install owner/repo"] --> B{"判断 Asset 类型"}
    B -->|单个 bin 文件| C["BinInstaller<br/>直接复制到 ~/.local/bin"]
    B -->|tar.gz/zip 等压缩包| D["ArchiveInstaller<br/>解压提取 bin 复制到 ~/.local/bin"]
    B -->|tar.gz (便携应用)| E["PortableAppInstaller<br/>解压到 ~/.local/app 生成快捷方式"]
    B -->|AppImage| F["AppImageInstaller<br/>赋权复制到 ~/.local/bin 或 ~/.local/app"]
    
    C --> G["AppRegistry<br/>记录元数据"]
    D --> G
    E --> G
    F --> G
    G --> H["dra update <app>"]
    G --> I["dra remove <app>"]
    H --> J["查询 GitHub 最新 Release<br/>比对版本 -> 重新执行 Install"]
    I --> K["读取 Registry 元数据<br/>执行对应 Uninstall 逻辑 -> 删除记录"]
```
---
### 二、 详细模块与修改计划
#### 1. 现状分析与 Bin/Archive 安装逻辑适配
dra 现有的 `dra download --install` **已经具备**解压 tar.gz 并提取单个或多个可执行文件的能力（在 `installer/archive_installer.rs` 中实现）。
**所需修改：**
- **默认目标路径调整**：目前可能需要用户手动指定安装目录。需要将默认的 Destination 修改为 `~/.local/bin/`（如果不存在则自动创建）。
- **不生成 Desktop**：现有逻辑本就不生成，直接复用即可。
- **可执行文件冲突处理**：如果 `~/.local/bin/` 下已存在同名文件，需要提供覆盖确认或备份机制。
#### 2. 状态管理模块：新增 `src/registry/` (核心)
为了支持 Update 和 Remove，必须记录安装的元信息。建议使用 JSON 文件作为轻量级数据库。
- **存储位置**：`~/.local/share/dra/installed_apps.json`
- **数据结构设计**：
```rust
// src/registry/app_record.rs
#[derive(Serialize, Deserialize, Clone)]
pub struct AppRecord {
    pub name: String,              // 应用名，如 "vscode"
    pub repo: String,              // 来源仓库，如 "microsoft/vscode"
    pub installed_version: String, // 当前安装的 Tag/Version
    pub install_type: InstallType, // 安装类型枚举
    pub installed_path: String,    // 安装主路径 (如 ~/.local/bin/ 或 ~/.local/app/vscode)
    pub executables: Vec<String>,  // 放在 ~/.local/bin 下的软链/可执行文件列表
    pub installed_at: String,      // ISO 时间戳
}
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum InstallType {
    Bin,            // 单独的 ELF 二进制
    ArchiveBin,     // tar.gz/zip 解压提取出的 bin
    PortableApp,    // 解压到 ~/.local/app 的便携应用（带 desktop）
    AppImage,       // AppImage 文件
}
```
- **Registry API**：提供 `add_record`, `remove_record`, `get_record`, `list_records` 等方法，在每次安装/卸载成功后调用。
#### 3. 统一安装入口与分发逻辑
在 `src/installer/install.rs` 中，根据下载的 Asset 扩展名或用户提供的参数，分发到不同的安装器，并在安装成功后写入 Registry。
| Asset 情况 | 触发安装器 | 安装路径 | Registry 记录的 InstallType |
|---|---|---|---|
| `*.AppImage` | AppImageInstaller | `~/.local/bin/` | `AppImage` |
| 单个无后缀文件 (如 `rg`) | BinInstaller | `~/.local/bin/` | `Bin` |
| `*.tar.gz` (常规工具) | ArchiveBinInstaller | `~/.local/bin/` | `ArchiveBin` |
| `*.tar.gz` (带 `--portable` 参数)| PortableAppInstaller | `~/.local/app/` | `PortableApp` |
#### 4. Remove 命令实现 (`dra remove <app_name>`)
新增 `dra remove <app_name>` CLI 子命令。
- **执行流程**：
  1. 从 `installed_apps.json` 读取 `app_name` 的记录。
  2. 根据 `install_type` 执行卸载逻辑：
     - `Bin` / `ArchiveBin` / `AppImage`：遍历 `executables` 列表，删除 `~/.local/bin/` 下的对应文件。
     - `PortableApp`：删除 `~/.local/app/<app_name>` 目录，删除 `~/.local/bin/<app_name>` 软链，删除 `~/.local/share/applications/<app_name>.desktop`。
  3. 从 `installed_apps.json` 中移除该记录。
  4. 输出卸载成功信息。
#### 5. Update 命令实现 (`dra update [app_name]`)
新增 `dra update` (更新所有) 或 `dra update <app_name>` (更新单个) 命令。
- **执行流程**：
  1. 读取 Registry，获取已安装应用列表。
  2. 并发或遍历调用 GitHub API (`src/github/releases.rs`) 获取最新 Release Tag。
  3. **比对版本**：如果 `latest_tag != installed_version`，则触发更新流程。
  4. **更新流程**：本质上是**先 Remove (旧版)，再 Install (新版)**，或者直接覆盖安装路径并更新 `installed_version` 字段。为了确保干净，推荐 "Remove -> Install -> 更新 Registry"。
---
### 三、 分阶段任务清单
#### 阶段 1：引入状态管理
- [ ] 新增 `src/registry/mod.rs` 和 `src/registry/app_record.rs`。
- [ ] 实现 `~/.local/share/dra/installed_apps.json` 的读写逻辑（使用 `serde_json`，dra 已有依赖）。
- [ ] 在现有的 `dra download --install` 流程末尾，增加写入 `AppRecord` 的逻辑（哪怕目前只记录基础信息）。
#### 阶段 2：Bin 与 Archive 的标准化安装
- [ ] 修改 `Destination` 逻辑，默认指向 `~/.local/bin`，若不存在则 `create_dir_all`。
- [ ] 完善可执行文件提取后，将文件名列表（`executables`）收集起来，写入 `AppRecord`。
- [ ] 新增 `AppImage` 安装支持（下载后 `chmod +x` 并移动到 `~/.local/bin`）。
#### 阶段 3：实现 Remove 命令
- [ ] 在 `src/cli/root_command.rs` 添加 `Remove { name: String }` 子命令。
- [ ] 新建 `src/cli/remove_handler.rs`。
- [ ] 根据上述设计的卸载逻辑，实现针对 `Bin`, `ArchiveBin`, `AppImage`, `PortableApp` 的删除函数。
- [ ] 删除完成后同步更新 Registry。
#### 阶段 4：实现 Update 命令
- [ ] 在 `src/cli/root_command.rs` 添加 `Update { name: Option<String> }` 子命令。
- [ ] 新建 `src/cli/update_handler.rs`。
- [ ] 实现"遍历本地记录 -> 请求 GitHub API 比对 Tag -> 触发下载 -> 覆盖安装 -> 更新本地版本号"的完整链路。
- [ ] 如果 `app_name` 存在，则只更新指定应用；如果不存在，列出所有应用并依次检查更新。
#### 阶段 5：整合 PortableApp (上一轮计划)
- [ ] 将原先设计的便携应用安装整合进新的架构中。
- [ ] 确保便携应用安装时也通过统一入口记录到 `AppRecord`，`install_type` 标记为 `PortableApp`。
- [ ] 确保 `remove` 命令能正确清理便携应用的 `.desktop`、`.service` 和 `~/.local/app` 目录。
#### 阶段 6：完善体验与测试
- [ ] 实现 `dra list` 命令，打印当前已安装的应用表格（名称、版本、类型、来源）。
- [ ] 添加处理 `~/.local/bin` 不在 PATH 中的友好提示。
- [ ] 编写单元测试：Registry 的增删改查、各种 `install_type` 的卸载逻辑。
### 四、 CLI 使用场景示例（最终形态）
```bash
# 1. 下载单个二进制工具
dra install BurntSushi/ripgrep
# -> 自动识别 tar.gz，解压提取 rg，放入 ~/.local/bin
# -> 记录: name="ripgrep", type=ArchiveBin, version="13.0.0"
# 2. 下载便携应用（带 GUI 的）
dra install microsoft/vscode --portable --bin bin/code
# -> 解压到 ~/.local/app/vscode，生成 desktop
# -> 记录: name="vscode", type=PortableApp, version="1.85.0"
# 3. 查看已安装
dra list
# Name      Version   Type          Repo
# ripgrep   13.0.0    ArchiveBin    BurntSushi/ripgrep
# vscode    1.85.0    PortableApp   microsoft/vscode
# 4. 检测并更新
dra update vscode
# -> Found new version: 1.86.0. Updating...
# -> Removed old version, downloading new asset, installing...
# 5. 卸载
dra remove ripgrep
# -> Removing executable /home/user/.local/bin/rg
# -> Record deleted.
```
### 五、 关键注意点
1. **并发安全**：`installed_apps.json` 的读写不需要数据库级别的锁，但 Update 命令并发拉取多个应用时，写入 Registry 需要在每个应用更新完成后串行写入，避免文件损坏。
2. **错误回滚**：在 Install 过程中（如复制文件失败），应清理已复制的半个文件；在 Remove 过程中，即使某个文件不存在也不应中断整个卸载流程，继续删除其他文件并清除记录。
3. **Tag 清洗**：GitHub 的 Tag 有时带有 `v` 前缀（如 `v1.0.0`），有时不带。比对版本时应做规范化处理，否则会导致频繁无效更新。
