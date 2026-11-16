# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_ss_belt_global_optspecs
    string join \n dry-run config= v/verbose h/help V/version
end

function __fish_ss_belt_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_ss_belt_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_ss_belt_using_subcommand
    set -l cmd (__fish_ss_belt_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c ss-belt -n "__fish_ss_belt_needs_command" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_needs_command" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -s V -l version -d 'Print version'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "region" -d 'Capture a region, save it and copy it'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "full" -d 'Capture every monitor'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "window" -d 'Capture the active window'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "monitor" -d 'Capture the current monitor'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "annotate" -d 'Capture a region and annotate it before saving'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "clipboard-edit" -d 'Edit the image currently on the clipboard'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "qr-scan" -d 'Decode a QR code from a region and copy the result'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "qr-make" -d 'Turn text into a QR image'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "ocr" -d 'Extract text from a region with tesseract'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "upload" -d 'Upload a file, or the last capture when no file is given'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "annotate-last" -d 'Annotate the last capture again'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "gif" -d 'Record a GIF (run again to stop)'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "video" -d 'Record a video (run again to stop)'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "edit" -d 'Annotate a file, or the clipboard image'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "qr" -d 'QR helpers'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "actions" -d 'List the built-in actions'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "steps" -d 'List every pipeline step and whether it works here'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "doctor" -d 'Probe the host and report what works'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "config" -d 'Show or check the configuration'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "history" -d 'Show recent history'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "hotkeys" -d 'Print hotkey snippets, or grab X11 hotkeys'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "daemon" -d 'Run or control the background daemon'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "trigger" -d 'Run an action in the daemon (or in-process when it is not running)'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "status" -d 'One-line status of the toolchain and daemon'
complete -c ss-belt -n "__fish_ss_belt_needs_command" -f -a "completion" -d 'Generate shell completions'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand region" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand region" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand region" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand region" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand full" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand full" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand full" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand full" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand window" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand window" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand window" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand window" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand monitor" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand monitor" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand monitor" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand monitor" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand annotate" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand annotate" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand annotate" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand annotate" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand clipboard-edit" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand clipboard-edit" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand clipboard-edit" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand clipboard-edit" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr-scan" -l file -d 'Decode this image instead of capturing the screen' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr-scan" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr-scan" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr-scan" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr-scan" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr-make" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr-make" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr-make" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr-make" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand ocr" -l file -d 'Read this image instead of capturing the screen' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand ocr" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand ocr" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand ocr" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand ocr" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand upload" -l destination -d 'Destination name from the config' -r
complete -c ss-belt -n "__fish_ss_belt_using_subcommand upload" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand upload" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand upload" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand upload" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand annotate-last" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand annotate-last" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand annotate-last" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand annotate-last" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand gif" -l duration -d 'Record for this many seconds, then convert' -r
complete -c ss-belt -n "__fish_ss_belt_using_subcommand gif" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand gif" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand gif" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand gif" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand video" -l duration -r
complete -c ss-belt -n "__fish_ss_belt_using_subcommand video" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand video" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand video" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand video" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand edit" -l file -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand edit" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand edit" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand edit" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand edit" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr; and not __fish_seen_subcommand_from scan make" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr; and not __fish_seen_subcommand_from scan make" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr; and not __fish_seen_subcommand_from scan make" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr; and not __fish_seen_subcommand_from scan make" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr; and not __fish_seen_subcommand_from scan make" -f -a "scan" -d 'Decode a QR code from the screen'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr; and not __fish_seen_subcommand_from scan make" -f -a "make" -d 'Encode text into a QR image'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr; and __fish_seen_subcommand_from scan" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr; and __fish_seen_subcommand_from scan" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr; and __fish_seen_subcommand_from scan" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr; and __fish_seen_subcommand_from scan" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr; and __fish_seen_subcommand_from make" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr; and __fish_seen_subcommand_from make" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr; and __fish_seen_subcommand_from make" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand qr; and __fish_seen_subcommand_from make" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand actions" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand actions" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand actions" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand actions" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand steps" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand steps" -l available -d 'Only print steps that can run on this host'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand steps" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand steps" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand steps" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand doctor" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand doctor" -l json
complete -c ss-belt -n "__fish_ss_belt_using_subcommand doctor" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand doctor" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand doctor" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand config" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand config" -l path -d 'Print the config file path'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand config" -l default -d 'Print the documented default config'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand config" -l check -d 'Parse and validate without running anything'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand config" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand config" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand config" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand history" -l limit -r
complete -c ss-belt -n "__fish_ss_belt_using_subcommand history" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand history" -l uploads -d 'Only uploads'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand history" -l clear -d 'Delete all history'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand history" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand history" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand history" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand hotkeys" -l print -d 'Print a snippet for this desktop (niri, sway, hyprland, gnome, kde)' -r
complete -c ss-belt -n "__fish_ss_belt_using_subcommand hotkeys" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand hotkeys" -l check -d 'Check that every configured accelerator parses'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand hotkeys" -l grab -d 'Grab the hotkeys on X11 (blocks)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand hotkeys" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand hotkeys" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand hotkeys" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and not __fish_seen_subcommand_from start stop status reload" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and not __fish_seen_subcommand_from start stop status reload" -l detach
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and not __fish_seen_subcommand_from start stop status reload" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and not __fish_seen_subcommand_from start stop status reload" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and not __fish_seen_subcommand_from start stop status reload" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and not __fish_seen_subcommand_from start stop status reload" -f -a "start" -d 'Start the daemon in the background'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and not __fish_seen_subcommand_from start stop status reload" -f -a "stop" -d 'Ask the daemon to exit'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and not __fish_seen_subcommand_from start stop status reload" -f -a "status" -d 'Print the daemon status'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and not __fish_seen_subcommand_from start stop status reload" -f -a "reload" -d 'Reload the config in a running daemon'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from start" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from start" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from start" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from start" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from stop" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from stop" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from stop" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from stop" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from status" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from status" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from status" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from status" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from reload" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from reload" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from reload" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand daemon; and __fish_seen_subcommand_from reload" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand trigger" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand trigger" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand trigger" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand trigger" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand status" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand status" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand status" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand status" -s h -l help -d 'Print help'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand completion" -l config -d 'Use an alternate config file' -r -F
complete -c ss-belt -n "__fish_ss_belt_using_subcommand completion" -l dry-run -d 'Show what would happen without capturing, uploading or writing files'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand completion" -s v -l verbose -d 'Increase log verbosity (-v for steps, -vv for debug)'
complete -c ss-belt -n "__fish_ss_belt_using_subcommand completion" -s h -l help -d 'Print help'
