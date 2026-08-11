use std::env;

use crate::{Result, cli::Shell};

pub fn print_init(shell: Shell) -> Result<()> {
    let script = match shell {
        Shell::Bash => BASH,
        Shell::Zsh => ZSH,
        Shell::Fish => FISH,
        Shell::Nu => NU,
    };
    let binary = env::current_exe()?.canonicalize()?;
    let binary = match shell {
        Shell::Nu => format!("{:?}", binary.to_string_lossy()),
        _ => format!("'{}'", binary.to_string_lossy().replace('\'', "'\\''")),
    };
    print!("{}", script.replace("__TERMINAL_HISTORY_BIN__", &binary));
    Ok(())
}

const BASH: &str = r#"__terminal_history_bin=__TERMINAL_HISTORY_BIN__
__terminal_history_ready=0
__terminal_history_initialized=0
__terminal_history_last=$HISTCMD
__terminal_history_read_command() {
    local history_line
    history_line="$(HISTTIMEFORMAT= builtin history 1)"
    [[ $history_line =~ ^[[:space:]]*[0-9]+[[:space:]]+(.*)$ ]] && printf '%s' "${BASH_REMATCH[1]}"
}
__terminal_history_preexec() {
    local last_status=$? function_name
    for function_name in "${FUNCNAME[@]:1}"; do
        [[ $function_name == __terminal_history_* ]] && return "$last_status"
    done
    (( __terminal_history_ready )) || return "$last_status"
    [[ $BASH_COMMAND == __terminal_history_* ]] && return "$last_status"
    __terminal_history_ready=0
    __terminal_history_command="$(__terminal_history_read_command)"
    __terminal_history_cwd="$PWD"
    return "$last_status"
}
__terminal_history_prompt() {
    local prompt_status=$?
    if (( ! __terminal_history_initialized )); then
        __terminal_history_initialized=1
        __terminal_history_last=$HISTCMD
        __terminal_history_ready=1
        return "$prompt_status"
    fi
    if [[ -z "$__terminal_history_command" && $HISTCMD != "$__terminal_history_last" ]]; then
        __terminal_history_command="$(__terminal_history_read_command)"
        : "${__terminal_history_cwd:=$PWD}"
    fi
    if [[ -n "$__terminal_history_command" ]]; then
        ("$__terminal_history_bin" add --command "$__terminal_history_command" --cwd "$__terminal_history_cwd" --shell bash --status "$__terminal_history_status" >/dev/null 2>&1 &)
        unset __terminal_history_command __terminal_history_cwd
    fi
    __terminal_history_last=$HISTCMD
    __terminal_history_ready=1
    return "$prompt_status"
}
if [[ ${BASH_VERSINFO[0]} -ge 5 ]]; then
    __terminal_history_prompt_commands=()
    for __terminal_history_item in "${PROMPT_COMMAND[@]}"; do
        [[ $__terminal_history_item == '__terminal_history_status=$?' || $__terminal_history_item == __terminal_history_prompt ]] || __terminal_history_prompt_commands+=("$__terminal_history_item")
    done
    PROMPT_COMMAND=('__terminal_history_status=$?' "${__terminal_history_prompt_commands[@]}" __terminal_history_prompt)
    unset __terminal_history_prompt_commands __terminal_history_item
else
    PROMPT_COMMAND="__terminal_history_status=\$?${PROMPT_COMMAND:+;$PROMPT_COMMAND};__terminal_history_prompt"
fi
if declare -p preexec_functions >/dev/null 2>&1; then
    [[ " ${preexec_functions[*]} " == *" __terminal_history_preexec "* ]] || preexec_functions+=(__terminal_history_preexec)
elif [[ -z $(trap -p DEBUG) ]]; then
    trap '__terminal_history_preexec' DEBUG
fi

__terminal_history_search() {
    local selected
    selected="$("$__terminal_history_bin" pick --query "$READLINE_LINE")"
    if [[ -n "$selected" ]]; then
        READLINE_LINE="$selected"
        READLINE_POINT=${#READLINE_LINE}
    fi
}
__terminal_history_up() {
    if [[ "$READLINE_LINE" != "${__terminal_history_selected-}" ]]; then
        __terminal_history_prefix="$READLINE_LINE"
        __terminal_history_offset=0
    fi
    local selected
    selected="$("$__terminal_history_bin" recall --prefix "$__terminal_history_prefix" --offset "$__terminal_history_offset")"
    if [[ -n "$selected" ]]; then
        READLINE_LINE="$selected"
        READLINE_POINT=${#READLINE_LINE}
        __terminal_history_selected="$selected"
        ((__terminal_history_offset++))
    fi
}
__terminal_history_down() {
    if (( __terminal_history_offset <= 1 )); then
        READLINE_LINE="$__terminal_history_prefix"
        READLINE_POINT=${#READLINE_LINE}
        __terminal_history_selected="$READLINE_LINE"
        __terminal_history_offset=0
        return
    fi
    ((__terminal_history_offset-=2))
    __terminal_history_up
}
bind -r "${TERMINAL_HISTORY_SEARCH_KEY:-\\C-r}" 2>/dev/null
bind -r "${TERMINAL_HISTORY_UP_KEY:-\\e[A}" 2>/dev/null
bind -r "${TERMINAL_HISTORY_DOWN_KEY:-\\e[B}" 2>/dev/null
bind -x "\"${TERMINAL_HISTORY_SEARCH_KEY:-\\C-r}\":__terminal_history_search"
bind -x "\"${TERMINAL_HISTORY_UP_KEY:-\\e[A}\":__terminal_history_up"
bind -x "\"${TERMINAL_HISTORY_DOWN_KEY:-\\e[B}\":__terminal_history_down"
"#;

const ZSH: &str = r#"__terminal_history_bin=__TERMINAL_HISTORY_BIN__
autoload -Uz add-zsh-hook
zmodload zsh/datetime 2>/dev/null
__terminal_history_preexec() {
    [[ -n "$1" ]] || return
    __terminal_history_command="$1"
    __terminal_history_cwd="$PWD"
    __terminal_history_started=${EPOCHREALTIME:-$SECONDS}
}
__terminal_history_precmd() {
    local command_status=$?
    if [[ -n "$__terminal_history_command" ]]; then
        local elapsed=$(( int((${EPOCHREALTIME:-$SECONDS} - __terminal_history_started) * 1000) ))
        "$__terminal_history_bin" add --command "$__terminal_history_command" --cwd "$__terminal_history_cwd" --shell zsh --status "$command_status" --duration "$elapsed" >/dev/null 2>&1 &
        unset __terminal_history_command
    fi
}
add-zsh-hook -d preexec __terminal_history_preexec 2>/dev/null
add-zsh-hook -d precmd __terminal_history_precmd 2>/dev/null
add-zsh-hook preexec __terminal_history_preexec
add-zsh-hook precmd __terminal_history_precmd

__terminal_history_search() {
    local selected="$("$__terminal_history_bin" pick --query "$BUFFER")"
    if [[ -n "$selected" ]]; then
        BUFFER="$selected"
        CURSOR=${#BUFFER}
    fi
    zle redisplay
}
__terminal_history_up() {
    if [[ "$BUFFER" != "${__terminal_history_selected-}" ]]; then
        __terminal_history_prefix="$BUFFER"
        __terminal_history_offset=0
    fi
    local selected="$("$__terminal_history_bin" recall --prefix "$__terminal_history_prefix" --offset "$__terminal_history_offset")"
    if [[ -n "$selected" ]]; then
        BUFFER="$selected"
        CURSOR=${#BUFFER}
        __terminal_history_selected="$selected"
        ((__terminal_history_offset++))
    fi
    zle redisplay
}
__terminal_history_down() {
    if (( __terminal_history_offset <= 1 )); then
        BUFFER="$__terminal_history_prefix"
        CURSOR=${#BUFFER}
        __terminal_history_selected="$BUFFER"
        __terminal_history_offset=0
        zle redisplay
        return
    fi
    ((__terminal_history_offset-=2))
    __terminal_history_up
}
zle -N __terminal_history_search
zle -N __terminal_history_up
zle -N __terminal_history_down
bindkey -r "${TERMINAL_HISTORY_SEARCH_KEY:-^R}" 2>/dev/null
bindkey -r "${TERMINAL_HISTORY_UP_KEY:-^[[A}" 2>/dev/null
bindkey -r "${TERMINAL_HISTORY_DOWN_KEY:-^[[B}" 2>/dev/null
bindkey "${TERMINAL_HISTORY_SEARCH_KEY:-^R}" __terminal_history_search
bindkey "${TERMINAL_HISTORY_UP_KEY:-^[[A}" __terminal_history_up
bindkey "${TERMINAL_HISTORY_DOWN_KEY:-^[[B}" __terminal_history_down
"#;

const FISH: &str = r#"set -g __terminal_history_bin __TERMINAL_HISTORY_BIN__
function __terminal_history_preexec --on-event fish_preexec
    test -n "$argv[1]"; or return
    set -g __terminal_history_command $argv[1]
    set -g __terminal_history_cwd $PWD
end
function __terminal_history_postexec --on-event fish_postexec
    set -l command_status $status
    if set -q __terminal_history_command
        command $__terminal_history_bin add --command "$__terminal_history_command" --cwd "$__terminal_history_cwd" --shell fish --status "$command_status" --duration "$CMD_DURATION" >/dev/null 2>&1 &
        set -e __terminal_history_command
        set -e __terminal_history_cwd
    end
end

function __terminal_history_search
    set -l selected (command $__terminal_history_bin pick --query (commandline))
    if test -n "$selected"
        commandline -- "$selected"
    end
    commandline -f repaint
end
function __terminal_history_up
    set -l line (commandline)
    if not set -q __terminal_history_selected; or test "$line" != "$__terminal_history_selected"
        set -g __terminal_history_prefix "$line"
        set -g __terminal_history_offset 0
    end
    set -l selected (command $__terminal_history_bin recall --prefix "$__terminal_history_prefix" --offset "$__terminal_history_offset")
    if test -n "$selected"
        commandline -- "$selected"
        set -g __terminal_history_selected "$selected"
        set -g __terminal_history_offset (math $__terminal_history_offset + 1)
    end
    commandline -f repaint
end
function __terminal_history_down
    if not set -q __terminal_history_offset
        return
    end
    if test "$__terminal_history_offset" -le 1
        commandline -- "$__terminal_history_prefix"
        set -g __terminal_history_selected "$__terminal_history_prefix"
        set -g __terminal_history_offset 0
        commandline -f repaint
        return
    end
    set -g __terminal_history_offset (math $__terminal_history_offset - 2)
    __terminal_history_up
end
bind --erase (set -q TERMINAL_HISTORY_SEARCH_KEY; and echo $TERMINAL_HISTORY_SEARCH_KEY; or echo '\cr') 2>/dev/null
bind --erase (set -q TERMINAL_HISTORY_UP_KEY; and echo $TERMINAL_HISTORY_UP_KEY; or echo '\e[A') 2>/dev/null
bind --erase (set -q TERMINAL_HISTORY_DOWN_KEY; and echo $TERMINAL_HISTORY_DOWN_KEY; or echo '\e[B') 2>/dev/null
bind (set -q TERMINAL_HISTORY_SEARCH_KEY; and echo $TERMINAL_HISTORY_SEARCH_KEY; or echo '\cr') __terminal_history_search
bind (set -q TERMINAL_HISTORY_UP_KEY; and echo $TERMINAL_HISTORY_UP_KEY; or echo '\e[A') __terminal_history_up
bind (set -q TERMINAL_HISTORY_DOWN_KEY; and echo $TERMINAL_HISTORY_DOWN_KEY; or echo '\e[B') __terminal_history_down
"#;

const NU: &str = r#"let __terminal_history_loaded = ($env.config.keybindings? | default [] | any {|binding| $binding.name? == terminal_history_search })
$env.__terminal_history_bin = __TERMINAL_HISTORY_BIN__
if not $__terminal_history_loaded {
    $env.config = ($env.config
        | upsert hooks.pre_execution (($env.config.hooks.pre_execution? | default []) | append {||
            let command = (commandline)
            if ($command | is-not-empty) {
                $env.__terminal_history_command = $command
                $env.__terminal_history_cwd = $env.PWD
            }
        })
        | upsert hooks.pre_prompt (($env.config.hooks.pre_prompt? | default []) | append {||
            if ($env.__terminal_history_command? | is-not-empty) {
                let command = $env.__terminal_history_command
                let cwd = $env.__terminal_history_cwd
                let status = ($env.LAST_EXIT_CODE? | default 0)
                let duration = ($env.CMD_DURATION_MS? | default 0)
                hide-env __terminal_history_command
                hide-env __terminal_history_cwd
                ^$env.__terminal_history_bin add --command $command --cwd $cwd --shell nushell --status $status --duration $duration | complete | ignore
                $env.LAST_EXIT_CODE = $status
                $env.CMD_DURATION_MS = $duration
            }
        }))
}
$env.config = ($env.config
    | upsert menus (($env.config.menus? | default [])
        | where not ($it.name? == terminal_history_menu)
        | append [{
            name: terminal_history_menu
            only_buffer_difference: false
            marker: "history> "
            type: {
                layout: list
                page_size: 10
            }
            style: {
                text: white
                selected_text: cyan_reverse
                description_text: yellow
            }
            source: {|buffer, position|
                let output = (^$env.__terminal_history_bin candidates --prefix $buffer | complete)
                if $output.exit_code != 0 {
                    []
                } else {
                    $output.stdout
                    | split row (char nul)
                    | where {|command| $command | is-not-empty }
                    | each {|command|
                        {
                            value: $command
                            span: { start: 0 end: $position }
                        }
                    }
                }
            }
        }])
    | upsert keybindings (($env.config.keybindings? | default [])
        | where not (
            ($it.name? in [terminal_history_search terminal_history_up terminal_history_down]) or
            ($it.modifier == control and $it.keycode == ($env.TERMINAL_HISTORY_SEARCH_KEY? | default char_r)) or
            ($it.modifier == none and ($it.keycode == ($env.TERMINAL_HISTORY_UP_KEY? | default up) or $it.keycode == ($env.TERMINAL_HISTORY_DOWN_KEY? | default down)))
        )
        | append [{
            name: terminal_history_search
            modifier: control
            keycode: ($env.TERMINAL_HISTORY_SEARCH_KEY? | default char_r)
            mode: [emacs vi_insert vi_normal]
            event: { send: executehostcommand, cmd: 'commandline edit (^$env.__terminal_history_bin pick --query (commandline))' }
        } {
            name: terminal_history_up
            modifier: none
            keycode: ($env.TERMINAL_HISTORY_UP_KEY? | default up)
            mode: [emacs vi_insert vi_normal]
            event: { send: executehostcommand, cmd: 'let line = (commandline); if ($env.__terminal_history_offset? | is-empty) or ($line != ($env.__terminal_history_selected? | default $line)) { $env.__terminal_history_prefix = $line; $env.__terminal_history_offset = 0; $env.__terminal_history_selected = $line }; let selected = (^$env.__terminal_history_bin recall --prefix $env.__terminal_history_prefix --offset $env.__terminal_history_offset); if ($selected | is-not-empty) { commandline edit $selected; $env.__terminal_history_selected = $selected; $env.__terminal_history_offset = $env.__terminal_history_offset + 1 }' }
        } {
            name: terminal_history_down
            modifier: none
            keycode: ($env.TERMINAL_HISTORY_DOWN_KEY? | default down)
            mode: [emacs vi_insert vi_normal]
            event: { send: executehostcommand, cmd: 'if ($env.__terminal_history_offset? | is-not-empty) { let offset = $env.__terminal_history_offset; if $offset <= 1 { commandline edit ($env.__terminal_history_prefix? | default ""); hide-env __terminal_history_selected; hide-env __terminal_history_offset } else { let next_offset = $offset - 2; let prefix = ($env.__terminal_history_prefix? | default ""); let selected = (^$env.__terminal_history_bin recall --prefix $prefix --offset $next_offset); if ($selected | is-not-empty) { commandline edit $selected; $env.__terminal_history_selected = $selected; $env.__terminal_history_offset = $next_offset + 1 } } }' }
        }]))
$env.config.hinter.closure = {|ctx|
    if $ctx.pos != ($ctx.line | str length) or ($ctx.line | is-empty) {
        ""
    } else {
        let candidate = (^$env.__terminal_history_bin recall --prefix $ctx.line)
        if ($candidate | is-empty) or not ($candidate | str starts-with $ctx.line) {
            ""
        } else {
            $candidate | str substring ($ctx.line | str length)..
        }
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_hook_records_command_directory_and_status() {
        for script in [BASH, ZSH, FISH, NU] {
            assert!(script.contains("--command"));
            assert!(script.contains("--cwd"));
            assert!(script.contains("--shell"));
            assert!(script.contains("--status"));
        }
        for script in [ZSH, FISH, NU] {
            assert!(script.contains("--duration"));
        }
    }

    #[test]
    fn hooks_append_and_avoid_duplicate_registration() {
        assert!(BASH.contains("PROMPT_COMMAND=('__terminal_history_status=$?'"));
        assert!(BASH.contains("__terminal_history_initialized=0"));
        assert!(ZSH.contains("add-zsh-hook -d preexec"));
        assert!(FISH.contains("--on-event fish_preexec"));
        assert!(FISH.contains("--on-event fish_postexec"));
        assert!(NU.contains("hooks.pre_execution") && NU.contains("| append"));
        assert!(NU.contains("let __terminal_history_loaded ="));
    }

    #[test]
    fn history_keys_replace_native_bindings() {
        assert!(BASH.contains("bind -r"));
        assert!(ZSH.contains("bindkey -r"));
        assert!(FISH.contains("bind --erase"));
        assert!(NU.contains("where not"));
    }

    #[test]
    fn nushell_history_hint_uses_recall() {
        assert!(NU.contains("$env.config.hinter.closure = {|ctx|"));
        assert!(NU.contains("recall --prefix $ctx.line"));
    }

    #[test]
    fn nushell_history_navigation_stays_in_reedline() {
        assert!(NU.contains("name: terminal_history_menu"));
        assert!(NU.contains("candidates --prefix $buffer | complete"));
        assert!(NU.contains("send: executehostcommand"));
        assert!(NU.contains("recall --prefix"));
        assert!(NU.contains("--offset"));
        assert!(!NU.contains("{ send: menuup }"));
        assert!(!NU.contains("{ send: menudown }"));
        assert!(NU.contains("__terminal_history_offset"));
        assert!(NU.contains("__terminal_history_selected"));
    }

    #[test]
    fn nushell_menu_and_bindings_are_replaced_idempotently() {
        assert!(NU.contains("where not ($it.name? == terminal_history_menu)"));
        assert!(NU.contains(
            "$it.name? in [terminal_history_search terminal_history_up terminal_history_down]"
        ));
    }
}
