# Computer CLI

A CLI for providing mouse and keyboard usage to agents on Wayland based desktops.

## Example usage

### Discover the desktop

```sh
computer outputs                    # list connected monitors (and their wl_output names)
computer windows                    # list toplevel windows via ext-foreign-toplevel-list-v1
```

### Screenshots

```sh
computer screenshot -o /tmp/desk.png             # capture the first output
computer screenshot --output DP-1 -o ./hidpi.png # capture a specific monitor
computer screenshot > /tmp/desk.png              # PNG to stdout
```

### Mouse

```sh
computer mouse move 960 540                    # absolute pixels on the first output
computer mouse move 100 100 --output HDMI-A-1  # absolute pixels on a named output
computer mouse move-rel 50 -20                 # relative dx,dy from the current position
computer mouse click                           # left click (default)
computer mouse click --button right            # right click
computer mouse click --count 2                 # double-click
computer mouse down --button left              # press and hold (for drag)
computer mouse up   --button left              # release
computer mouse scroll 0 5                      # scroll down 5 ticks
computer mouse scroll -3 0                     # scroll left 3 ticks
```

### Keyboard

```sh
computer key tap Return                  # press + release
computer key tap Escape
computer key chord "ctrl+c"              # send a chord
computer key chord "ctrl+shift+t"        # reopen last tab
computer key down ctrl                   # hold a modifier
computer key up   ctrl                   # release it
```

### Typing text

```sh
computer type "hello, world"             # literal string with default 8ms inter-key delay
computer type --delay-ms 30 "slower for picky inputs"
```

### Pacing automations

```sh
computer key chord "super+t"             # open a terminal
computer sleep 250                       # let it settle
computer type "echo hi"
computer key tap Return
```

### Drag-and-drop pattern

```sh
computer mouse move 200 300
computer mouse down --button left
computer mouse move 600 300              # drag
computer mouse up   --button left
```

## Notes

- All commands target the active Wayland session. If you need X11 automation, use `xdotool` instead.
- Coordinates for `mouse move` are absolute pixels on a specific output; pass `--output` when you have multiple monitors.
- `screenshot` defaults to the first output if `--output` is omitted.
- `type` synthesises one keystroke per character; for long strings consider raising `--delay-ms` if the receiving app drops events.
