#compdef ss-belt

autoload -U is-at-least

_ss-belt() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
":: :_ss-belt_commands" \
"*::: :->ss-belt" \
&& ret=0
    case $state in
    (ss-belt)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ss-belt-command-$line[1]:"
        case $line[1] in
            (region)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(full)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(window)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(monitor)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(annotate)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(clipboard-edit)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(qr-scan)
_arguments "${_arguments_options[@]}" : \
'--file=[Decode this image instead of capturing the screen]:FILE:_files' \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(qr-make)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
'::text -- Text to encode; reads the clipboard when omitted:_default' \
&& ret=0
;;
(ocr)
_arguments "${_arguments_options[@]}" : \
'--file=[Read this image instead of capturing the screen]:FILE:_files' \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(upload)
_arguments "${_arguments_options[@]}" : \
'--destination=[Destination name from the config]:DESTINATION:_default' \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
'::file:_files' \
&& ret=0
;;
(annotate-last)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(gif)
_arguments "${_arguments_options[@]}" : \
'--duration=[Record for this many seconds, then convert]:DURATION:_default' \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(video)
_arguments "${_arguments_options[@]}" : \
'--duration=[]:DURATION:_default' \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(edit)
_arguments "${_arguments_options[@]}" : \
'--file=[]:FILE:_files' \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(qr)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_ss-belt__subcmd__qr_commands" \
"*::: :->qr" \
&& ret=0

    case $state in
    (qr)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ss-belt-qr-command-$line[1]:"
        case $line[1] in
            (scan)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(make)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
'::text:_default' \
&& ret=0
;;
        esac
    ;;
esac
;;
(actions)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(steps)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--available[Only print steps that can run on this host]' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(doctor)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--json[]' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(config)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--path[Print the config file path]' \
'--default[Print the documented default config]' \
'--check[Parse and validate without running anything]' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(history)
_arguments "${_arguments_options[@]}" : \
'--limit=[]:LIMIT:_default' \
'--config=[Use an alternate config file]:FILE:_files' \
'--uploads[Only uploads]' \
'--clear[Delete all history]' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(hotkeys)
_arguments "${_arguments_options[@]}" : \
'--print=[Print a snippet for this desktop (niri, sway, hyprland, gnome, kde)]:PRINT:_default' \
'--config=[Use an alternate config file]:FILE:_files' \
'--check[Check that every configured accelerator parses]' \
'--grab[Grab the hotkeys on X11 (blocks)]' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(daemon)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--detach[]' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_ss-belt__subcmd__daemon_commands" \
"*::: :->daemon" \
&& ret=0

    case $state in
    (daemon)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ss-belt-daemon-command-$line[1]:"
        case $line[1] in
            (start)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(stop)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(reload)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
        esac
    ;;
esac
;;
(trigger)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
':action -- Action name, e.g. region, qr_scan, gif:_default' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(completion)
_arguments "${_arguments_options[@]}" : \
'--config=[Use an alternate config file]:FILE:_files' \
'--dry-run[Show what would happen without capturing, uploading or writing files]' \
'*-v[Increase log verbosity (-v for steps, -vv for debug)]' \
'*--verbose[Increase log verbosity (-v for steps, -vv for debug)]' \
'-h[Print help]' \
'--help[Print help]' \
':shell:(bash elvish fish powershell zsh)' \
&& ret=0
;;
        esac
    ;;
esac
}

(( $+functions[_ss-belt_commands] )) ||
_ss-belt_commands() {
    local commands; commands=(
'region:Capture a region, save it and copy it' \
'full:Capture every monitor' \
'window:Capture the active window' \
'monitor:Capture the current monitor' \
'annotate:Capture a region and annotate it before saving' \
'clipboard-edit:Edit the image currently on the clipboard' \
'qr-scan:Decode a QR code from a region and copy the result' \
'qr-make:Turn text into a QR image' \
'ocr:Extract text from a region with tesseract' \
'upload:Upload a file, or the last capture when no file is given' \
'annotate-last:Annotate the last capture again' \
'gif:Record a GIF (run again to stop)' \
'video:Record a video (run again to stop)' \
'edit:Annotate a file, or the clipboard image' \
'qr:QR helpers' \
'actions:List the built-in actions' \
'steps:List every pipeline step and whether it works here' \
'doctor:Probe the host and report what works' \
'config:Show or check the configuration' \
'history:Show recent history' \
'hotkeys:Print hotkey snippets, or grab X11 hotkeys' \
'daemon:Run or control the background daemon' \
'trigger:Run an action in the daemon (or in-process when it is not running)' \
'status:One-line status of the toolchain and daemon' \
'completion:Generate shell completions' \
    )
    _describe -t commands 'ss-belt commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__actions_commands] )) ||
_ss-belt__subcmd__actions_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt actions commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__annotate_commands] )) ||
_ss-belt__subcmd__annotate_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt annotate commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__annotate-last_commands] )) ||
_ss-belt__subcmd__annotate-last_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt annotate-last commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__clipboard-edit_commands] )) ||
_ss-belt__subcmd__clipboard-edit_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt clipboard-edit commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__completion_commands] )) ||
_ss-belt__subcmd__completion_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt completion commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__config_commands] )) ||
_ss-belt__subcmd__config_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt config commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__daemon_commands] )) ||
_ss-belt__subcmd__daemon_commands() {
    local commands; commands=(
'start:Start the daemon in the background' \
'stop:Ask the daemon to exit' \
'status:Print the daemon status' \
'reload:Reload the config in a running daemon' \
    )
    _describe -t commands 'ss-belt daemon commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__daemon__subcmd__reload_commands] )) ||
_ss-belt__subcmd__daemon__subcmd__reload_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt daemon reload commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__daemon__subcmd__start_commands] )) ||
_ss-belt__subcmd__daemon__subcmd__start_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt daemon start commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__daemon__subcmd__status_commands] )) ||
_ss-belt__subcmd__daemon__subcmd__status_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt daemon status commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__daemon__subcmd__stop_commands] )) ||
_ss-belt__subcmd__daemon__subcmd__stop_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt daemon stop commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__doctor_commands] )) ||
_ss-belt__subcmd__doctor_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt doctor commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__edit_commands] )) ||
_ss-belt__subcmd__edit_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt edit commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__full_commands] )) ||
_ss-belt__subcmd__full_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt full commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__gif_commands] )) ||
_ss-belt__subcmd__gif_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt gif commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__history_commands] )) ||
_ss-belt__subcmd__history_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt history commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__hotkeys_commands] )) ||
_ss-belt__subcmd__hotkeys_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt hotkeys commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__monitor_commands] )) ||
_ss-belt__subcmd__monitor_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt monitor commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__ocr_commands] )) ||
_ss-belt__subcmd__ocr_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt ocr commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__qr_commands] )) ||
_ss-belt__subcmd__qr_commands() {
    local commands; commands=(
'scan:Decode a QR code from the screen' \
'make:Encode text into a QR image' \
    )
    _describe -t commands 'ss-belt qr commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__qr__subcmd__make_commands] )) ||
_ss-belt__subcmd__qr__subcmd__make_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt qr make commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__qr__subcmd__scan_commands] )) ||
_ss-belt__subcmd__qr__subcmd__scan_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt qr scan commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__qr-make_commands] )) ||
_ss-belt__subcmd__qr-make_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt qr-make commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__qr-scan_commands] )) ||
_ss-belt__subcmd__qr-scan_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt qr-scan commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__region_commands] )) ||
_ss-belt__subcmd__region_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt region commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__status_commands] )) ||
_ss-belt__subcmd__status_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt status commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__steps_commands] )) ||
_ss-belt__subcmd__steps_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt steps commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__trigger_commands] )) ||
_ss-belt__subcmd__trigger_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt trigger commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__upload_commands] )) ||
_ss-belt__subcmd__upload_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt upload commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__video_commands] )) ||
_ss-belt__subcmd__video_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt video commands' commands "$@"
}
(( $+functions[_ss-belt__subcmd__window_commands] )) ||
_ss-belt__subcmd__window_commands() {
    local commands; commands=()
    _describe -t commands 'ss-belt window commands' commands "$@"
}

if [ "$funcstack[1]" = "_ss-belt" ]; then
    _ss-belt "$@"
else
    compdef _ss-belt ss-belt
fi
