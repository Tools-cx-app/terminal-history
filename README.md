# terminal-history

一个由官方 [`turso`](https://crates.io/crates/turso) Rust SDK 和 `clap` 驱动的跨 Shell
命令历史工具。默认使用本地 SQLite；设置 Turso Cloud 环境变量后，本地数据库会作为
副本同步到云端。

## 安装

```sh
cargo install --path .
```

默认数据库位于 `~/.local/share/terminal-history/history.db`。启用 Turso Cloud：

```sh
export TURSO_DATABASE_URL='libsql://your-database.turso.io'
export TURSO_AUTH_TOKEN='your-token'
# 可选：修改本地 SQLite/副本位置
export HISTORY_DATABASE_PATH="$HOME/.local/share/terminal-history/history.db"
```

## Shell 集成

将对应命令加入 Shell 配置：

```sh
# Bash: ~/.bashrc
eval "$(terminal-history init bash)"

# Zsh: ~/.zshrc
eval "$(terminal-history init zsh)"

# Fish: ~/.config/fish/config.fish
terminal-history init fish | source
```

Nushell 的 `config.nu`：

```nu
terminal-history init nu | save --force ~/.cache/terminal-history.nu
source ~/.cache/terminal-history.nu
```

Hook 会保留已有 Shell hook，并在命令完成后异步写入命令、时间、工作目录、Shell、
主机名、退出码和耗时。Bash 原生 history 不提供可靠耗时，因此其耗时为空。

| Shell | 开始 hook | 结束 hook | 记录内容 |
| --- | --- | --- | --- |
| Bash | `DEBUG` / `preexec_functions` | `PROMPT_COMMAND` | 执行前目录、命令、退出码 |
| Zsh | `preexec` | `precmd` | 执行前目录、命令、退出码、耗时 |
| Fish | `fish_preexec` | `fish_postexec` | 执行前目录、命令、退出码、耗时 |
| Nushell | `pre_execution` | `pre_prompt` | 执行前目录、命令、退出码、耗时 |

命令文本允许重复；每次执行都会保留独立记录。执行时间使用唯一纳秒时间戳，极端
碰撞时会在数据库内递增 `1ns`。
生成的初始化脚本会记录当前 `terminal-history` 可执行文件的绝对路径，因此从项目内
运行 `target/debug/terminal-history init nu` 也不要求该命令已加入 `PATH`。移动或重新
安装二进制后需要重新生成初始化脚本。键绑定产生的 `commandline edit`、`pick`、
`recall` 等内部命令会在写入入口自动忽略。

默认键绑定：

| 按键 | 行为 |
| --- | --- |
| `Ctrl-R` | 使用内置 Ratatui 界面交互搜索当前目录的历史 |
| `↑` / `↓` | 根据当前命令行前缀浏览当前目录的历史 |

Ratatui 选择器以内联面板显示在当前提示符下方，类似 `fzf --height`；可直接输入关键词
过滤，`↑` / `↓` 或 `PageUp` / `PageDown` 移动，
`Enter` 选择，`Esc` 或 `Ctrl-C` 取消，`Ctrl-U` 清空查询。记录按纳秒时间戳从新到旧
排序；宽终端显示本地执行时间，窄终端自动隐藏时间列。

可在执行 `terminal-history init ...` 前覆盖按键。不同 Shell 沿用自身的按键表示法：

```sh
# Bash，例如改为 Ctrl-X
export TERMINAL_HISTORY_SEARCH_KEY='\C-x'

# Zsh
export TERMINAL_HISTORY_SEARCH_KEY='^X'

# Fish
set -gx TERMINAL_HISTORY_SEARCH_KEY '\cx'
```

```nu
# Nushell，值使用 Reedline keycode
$env.TERMINAL_HISTORY_SEARCH_KEY = 'char_x'
```

同样可设置 `TERMINAL_HISTORY_UP_KEY`、`TERMINAL_HISTORY_DOWN_KEY`。默认选择器不依赖
`fzf`；如需使用外部选择器，可设置 `TERMINAL_HISTORY_SELECTOR=fzf`。

## CLI

```sh
terminal-history list
terminal-history list --limit 100 --cwd /path/to/project
terminal-history list --all
terminal-history search docker --limit 20
terminal-history search docker --all
terminal-history add --command 'cargo test' --cwd "$PWD" --shell bash --status 0 --duration 1200
terminal-history compact
terminal-history --help
```

`compact` 会回收 SQLite 未使用的页面，适合删除或大量迁移历史记录后手动执行；它不会删除任何历史记录。

`list` 和 `search` 默认只查询当前工作目录；传入 `--cwd` 可查询指定目录，`--all`
可跨目录查询。

命令内容会同步到配置的数据库，可能包含令牌或密码；敏感命令请勿直接写在命令行中。
