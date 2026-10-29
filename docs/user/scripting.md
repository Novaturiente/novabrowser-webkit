# Scripting and external tools

novabrowser-webkit has no extension system. Instead, other programs can control it,
and it can start other programs.

## Controlling a running browser

```sh
novabrowser-webkit --cmd 'open -t https://example.com'    # run any command
novabrowser-webkit --cmd 'tab-next ;; reload'
novabrowser-webkit --query state                          # JSON: windows, tabs, active tab, mode
novabrowser-webkit --key '<Down><Return>'                 # press keys, as if typed in the browser
novabrowser-webkit https://example.com                    # opens as a tab in the running instance
novabrowser-webkit                                        # opens a new window
```

`--cmd` and `--key` print `ok`; errors from the command show in the tab bar,
and a malformed key string prints `error: …`. `--key` takes key notation and
feeds the keys to the browser's own key handling (modes, bindings, hints,
prompts), not to the page. These talk to the instance of the same
profile over a Unix socket (`~/.local/share/novabrowser-webkit/ipc`); add
`--profile=name` for another profile. If nothing is running, they exit
with status 1.

Examples: bind a desktop shortcut to `novabrowser-webkit --cmd 'open -t {clipboard}'`,
or open links from a terminal or chat client into your current window.

## Starting programs

```
:spawn mpv {url}                 play the current page in mpv
:spawn -o ls -la ~/Downloads     show a program's output in a new tab
:hint links spawn mpv {hint-url} pick a link, play it
```

`spawn` never blocks the browser. Placeholders like `{url}`, `{title}` and
`{clipboard}` are replaced first.

## Userscripts

A userscript is any executable in `~/.config/novabrowser-webkit/userscripts/`
(or `~/.local/share/novabrowser-webkit/userscripts/`). Run it with `:spawn -u name
[args]` or bind it:

```ts
bind(",r", "spawn -u readability");
```

It gets the same environment as a qutebrowser userscript, so many
qutebrowser userscripts work unchanged:

| Variable | Contents |
|---|---|
| `QUTE_MODE` | `command` (or `hints` from a hint target) |
| `QUTE_URL`, `QUTE_TITLE` | current page |
| `QUTE_SELECTED_TEXT` | selected text |
| `QUTE_HTML`, `QUTE_TEXT` | paths of files with the page's HTML and text |
| `QUTE_FIFO` | a pipe: each line you write to it runs as a command |
| `QUTE_COUNT` | the count, if one was given |
| `QUTE_COMMANDLINE_TEXT` | text in the command line |
| `QUTE_CONFIG_DIR`, `QUTE_DATA_DIR`, `QUTE_DOWNLOAD_DIR` | directories |
| `QUTE_USERSCRIPT`, `QUTE_VERSION` | script path, browser version |

Commands written to `$QUTE_FIFO` run in the tab the script was started from
(dropped if that tab was closed meanwhile).

This one saves the page's URL to a reading list:

```sh
#!/bin/sh
echo "$QUTE_URL $QUTE_TITLE" >> ~/reading-list.txt
echo "message-info 'Saved to reading list'" >> "$QUTE_FIFO"
```

## Typing into pages

- `:insert-text hello` types into the focused field.
- `:fake-key <Ctrl-a>` or `:fake-key hello<Return>` sends key presses in
  [key notation](keybindings.md#key-notation); `\<` is a literal `<`.

WebKit has no way to send real input from the browser, so in this edition
neither is a real key press:

- `insert-text` inserts the text at the cursor with WebKit's editing
  commands. No key events are sent.
- `fake-key` sends JavaScript `keydown`/`keyup` events to the focused
  element. Pages see them with `isTrusted == false`, and they type nothing
  by themselves. The browser adds what it can: printable characters are
  inserted as text, `Backspace`, `Delete`, arrows, `Home`, `End` and
  `Ctrl-a`/`c`/`x`/`v`/`z` run the matching editing command, `Tab` moves
  focus to the next field, and `Return` in a form field submits the form.

That covers filling login and other simple forms from a script. Sites that
check for trusted input ignore it. (The CEF edition sends real input.)

## Web Inspector

`F12` (`:devtools`) opens WebKit's Web Inspector for the current tab.
There is no remote debugging port (Chrome DevTools Protocol) in this
edition; for automation use `--cmd`, `--query` and `--key`.