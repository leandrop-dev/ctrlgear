# Button Mapping

`steelctl buttons "<mapping>"` remaps the physical buttons on a supported mouse. This document covers the mapping string format, available actions, and the on-device protocol encoding, implemented in `src/buttons.rs`.

## Format

```
buttonN=action[,buttonN=action...]
```

- Only buttons you name are changed; any button not mentioned keeps its **current default** for that slot (see [Default mapping](#default-mapping)).
- Whitespace around `,` and `=` is ignored.
- The special value `"default"` resets all six buttons to their factory mapping and ignores any other assignments in the string.

```bash
steelctl buttons "button3=scrollup,button6=dpi"
steelctl buttons "default"
```

## Buttons

Buttons accept several aliases, case-insensitively (`src/buttons.rs:47-56`):

| Button   | Aliases              |
|----------|-----------------------|
| Button 1 | `button1`, `btn1`, `1` |
| Button 2 | `button2`, `btn2`, `2` |
| Button 3 | `button3`, `btn3`, `3` |
| Button 4 | `button4`, `btn4`, `4` |
| Button 5 | `button5`, `btn5`, `5` |
| Button 6 | `button6`, `btn6`, `6` |

On the Rival 3 Wireless, buttons 1–5 are the standard left/right/middle/side buttons; button 6 is the DPI-switch button on the underside of the mouse.

## Actions

Actions also accept several aliases (`src/buttons.rs:99-116`):

| Action | Aliases | Effect |
|--------|---------|--------|
| Mouse button remap | `button1`–`button6` (or `btn1`–`6`, `1`–`6`) | Remaps the physical button to act as a different mouse button |
| DPI switch | `dpi`, `dpiswitch`, `dpi_switch` | Cycles through configured DPI presets |
| Scroll up | `scrollup`, `scroll_up` | Emulates a scroll-wheel-up event |
| Scroll down | `scrolldown`, `scroll_down` | Emulates a scroll-wheel-down event |
| Disabled | `disabled`, `disable`, `none` | Button does nothing |
| Default | `default` | Keeps the factory action for that specific button slot |

## Examples

```bash
# Make the side buttons scroll instead of navigating back/forward
steelctl buttons "button4=scrollup,button5=scrolldown"

# Swap left and right click
steelctl buttons "button1=button2,button2=button1"

# Disable the DPI button
steelctl buttons "button6=disabled"

# Reset everything to factory defaults
steelctl buttons "default"
```

## Default mapping

`ButtonMapping::default_mapping` (`src/buttons.rs:127-138`):

| Button   | Default action |
|----------|----------------|
| Button 1 | Button 1 (left click) |
| Button 2 | Button 2 (right click) |
| Button 3 | Button 3 (middle click) |
| Button 4 | Button 4 |
| Button 5 | Button 5 |
| Button 6 | DPI switch |

## Protocol encoding

Each button's action encodes to a 5-byte field (`src/buttons.rs:78-90`); the full mapping is 6 buttons × 5 bytes = 30 bytes, sent after the `BUTTON_MAPPING` command byte (`0x19`, `src/protocol.rs:33`):

| Action | Encoded bytes |
|--------|---------------|
| Disabled | `00 00 00 00 00` |
| Mouse button N | `<N> 00 00 00 00` (button ID, 1–6) |
| DPI switch | `30 00 00 00 00` |
| Scroll up | `31 00 00 00 00` |
| Scroll down | `32 00 00 00 00` |

`ButtonAction::Default` is not sent as-is — it's resolved to the factory action for that button slot before encoding (`src/buttons.rs:163-169`), so the device always receives a concrete action.

Remember to run `steelctl save` after changing button mapping to persist it to device memory — without it, the mapping reverts on unplug/replug.
