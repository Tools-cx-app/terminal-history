# terminal-history

[English](README.md)

一个由官方 [`turso`](https://crates.io/crates/turso) Rust SDK 和 `clap` 驱动的跨
Shell 命令历史工具。默认使用本地 SQLite 数据库，也可与 Turso Cloud 同步。

## 功能

- 支持 Bash、Zsh、Fish 和 Nushell。
- 保留每次命令执行记录，包括重复命令。
- 默认按当前工作目录筛选历史记录。
- 支持使用 `↑` 和 `↓` 按前缀浏览历史，以及通过 `Ctrl-R` 打开内联 Ratatui 搜索界面。
- 保留已有 Shell hook，并记录命令、工作目录、Shell、主机名、退出码及可用的耗时信息。
- 使用本地 SQLite，并可选择与 Turso Cloud 同步。

## 安装

```sh
cargo install --path .
```

默认数据库路径为 `~/.local/share/terminal-history/history.db`。

## Shell 集成

将对应命令添加到 Shell 配置文件：

```sh
# Bash: ~/.bashrc
eval "$(terminal-history init bash)"

# Zsh: ~/.zshrc
eval "$(terminal-history init zsh)"

# Fish: ~/.config/fish/config.fish
terminal-history init fish | source
```

Nushell 用户请将以下内容添加到 `config.nu`：

```nu
terminal-history init nu | save --force ~/.cache/terminal-history.nu
source ~/.cache/terminal-history.nu
```

生成的集成脚本会保存可执行文件的绝对路径，因此移动或重新安装二进制文件后需要重新生成。

## 使用

```sh
terminal-history list
terminal-history list --limit 100 --cwd /path/to/project
terminal-history list --all
terminal-history search docker --limit 20
terminal-history search docker --all
terminal-history compact
terminal-history --help
```

`list` 和 `search` 默认只查询当前目录。使用 `--cwd` 可指定其他目录，使用 `--all`
可跨目录查询。`compact` 用于回收 SQLite 未使用的页面，不会删除历史记录。

## 配置

启用 Turso Cloud 同步：

```sh
export TURSO_DATABASE_URL='libsql://your-database.turso.io'
export TURSO_AUTH_TOKEN='your-token'
```

以下环境变量为可选配置：

| 变量 | 用途 |
| --- | --- |
| `HISTORY_DATABASE_PATH` | 修改本地 SQLite 数据库或副本路径。 |
| `TERMINAL_HISTORY_SEARCH_KEY` | 修改 `Ctrl-R` 搜索按键。 |
| `TERMINAL_HISTORY_UP_KEY` | 修改向上浏览历史的按键。 |
| `TERMINAL_HISTORY_DOWN_KEY` | 修改向下浏览历史的按键。 |
| `TERMINAL_HISTORY_SELECTOR` | 使用 `fzf` 等外部选择器。 |

按键表示法遵循各 Shell 自身的约定。内置选择器不依赖 `fzf`。

## 安全

同步到已配置数据库的命令可能包含密码、令牌或其他敏感信息。请避免直接在命令行中输入密钥。
