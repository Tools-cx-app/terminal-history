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
__terminal_history_last=$HISTCMD
__terminal_history_prompt() {
    local command_status=$? command
    if (( HISTCMD != __terminal_history_last )); then
        command="$(builtin fc -ln -1)"
        "$__terminal_history_bin" add --command "$command" --cwd "$PWD" --shell bash --status "$command_status" >/dev/null 2>&1 &
        __terminal_history_last=$HISTCMD
    fi
}
if [[ ${BASH_VERSINFO[0]} -ge 5 ]]; then
    PROMPT_COMMAND+=(__terminal_history_prompt)
else
    PROMPT_COMMAND="__terminal_history_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
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
bind -x "\"${TERMINAL_HISTORY_SEARCH_KEY:-\\C-r}\":__terminal_history_search"
bind -x "\"${TERMINAL_HISTORY_UP_KEY:-\\e[A}\":__terminal_history_up"
bind -x "\"${TERMINAL_HISTORY_DOWN_KEY:-\\e[B}\":__terminal_history_down"
"#;

const ZSH: &str = r#"__terminal_history_bin=__TERMINAL_HISTORY_BIN__
autoload -Uz add-zsh-hook
zmodload zsh/datetime 2>/dev/null
__terminal_history_preexec() {
    __terminal_history_command="$1"
    __terminal_history_cwd="$PWD"
    __terminal_history_started=${EPOCHREALTIME:-$SECONDS}
}
__terminal_history_precmd() {
    local command_status=$?
    if [[ -n "$__terminal_history_command" ]]; then
        local elapsed=$(( (${EPOCHREALTIME:-$SECONDS} - __terminal_history_started) * 1000 ))
        "$__terminal_history_bin" add --command "$__terminal_history_command" --cwd "$__terminal_history_cwd" --shell zsh --status "$command_status" --duration "$elapsed" >/dev/null 2>&1 &
        unset __terminal_history_command
    fi
}
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
bindkey "${TERMINAL_HISTORY_SEARCH_KEY:-^R}" __terminal_history_search
bindkey "${TERMINAL_HISTORY_UP_KEY:-^[[A}" __terminal_history_up
bindkey "${TERMINAL_HISTORY_DOWN_KEY:-^[[B}" __terminal_history_down
"#;

const FISH: &str = r#"set -g __terminal_history_bin __TERMINAL_HISTORY_BIN__
function __terminal_history_preexec --on-event fish_preexec
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
bind (set -q TERMINAL_HISTORY_SEARCH_KEY; and echo $TERMINAL_HISTORY_SEARCH_KEY; or echo '\cr') __terminal_history_search
bind (set -q TERMINAL_HISTORY_UP_KEY; and echo $TERMINAL_HISTORY_UP_KEY; or echo '\e[A') __terminal_history_up
bind (set -q TERMINAL_HISTORY_DOWN_KEY; and echo $TERMINAL_HISTORY_DOWN_KEY; or echo '\e[B') __terminal_history_down
"#;

const NU: &str = r#"$env.__terminal_history_bin = __TERMINAL_HISTORY_BIN__
$env.config = ($env.config
    | upsert hooks.pre_execution (($env.config.hooks.pre_execution? | default []) | append {||
        $env.__terminal_history_command = (commandline)
        $env.__terminal_history_cwd = $env.PWD
    })
    | upsert hooks.pre_prompt (($env.config.hooks.pre_prompt? | default []) | append {||
        if ($env.__terminal_history_command? | is-not-empty) {
            let command = $env.__terminal_history_command
            let cwd = $env.__terminal_history_cwd
            let status = ($env.LAST_EXIT_CODE? | default 0)
            let duration = ($env.CMD_DURATION_MS? | default 0)
            hide-env __terminal_history_command
            hide-env __terminal_history_cwd
            ^$env.__terminal_history_bin add --command $command --cwd $cwd --shell nushell --status $status --duration $duration
        }
    })
    | upsert keybindings (($env.config.keybindings? | default []) | append [{
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
        event: { send: executehostcommand, cmd: 'let line = (commandline); if $line != ($env.__terminal_history_selected? | default "") { $env.__terminal_history_prefix = $line; $env.__terminal_history_offset = 0 }; let selected = (^$env.__terminal_history_bin recall --prefix $env.__terminal_history_prefix --offset $env.__terminal_history_offset); if ($selected | is-not-empty) { commandline edit $selected; $env.__terminal_history_selected = $selected; $env.__terminal_history_offset += 1 }' }
    } {
        name: terminal_history_down
        modifier: none
        keycode: ($env.TERMINAL_HISTORY_DOWN_KEY? | default down)
        mode: [emacs vi_insert vi_normal]
        event: { send: executehostcommand, cmd: 'if ($env.__terminal_history_offset? | default 0) <= 1 { let prefix = ($env.__terminal_history_prefix? | default ""); commandline edit $prefix; $env.__terminal_history_selected = $prefix; $env.__terminal_history_offset = 0 } else { $env.__terminal_history_offset -= 2; let selected = (^$env.__terminal_history_bin recall --prefix $env.__terminal_history_prefix --offset $env.__terminal_history_offset); commandline edit $selected; $env.__terminal_history_selected = $selected; $env.__terminal_history_offset += 1 }' }
    }]))
"#;
