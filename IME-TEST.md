# macOS Korean IME Test

This note describes the manual verification flow used for the macOS Korean
2-Set IME first-character composition fix, plus the follow-up fix that
rewrites confirm-key handling (`insertText` after a commit) to be based on a
protocol signal instead of guessing at the physical key's character.

The important part is to send real key events through the focused winit window.
Do not paste `한글`, and do not use a text-injection helper that bypasses IME
composition.

## 1. Run the sample

From the repository root:

```sh
cargo run --example ime_textbox
```

The sample opens a window named `IME textbox`. It logs every IME event to the
terminal and mirrors the committed buffer in the window title.

## 2. Focus the sample window

The `ime_textbox` sample is a raw executable, not an app bundle. Some UI
automation tools may not find it by bundle identifier. Use System Events if
needed:

```sh
osascript -e 'tell application "System Events" to set frontmost of application process "ime_textbox" to true'
osascript -e 'tell application "System Events" to get {name, frontmost} of application process "ime_textbox"'
```

Expected output:

```text
ime_textbox, true
```

## 3. Switch to Korean 2-Set

If the input source is already Korean 2-Set, skip this step. Otherwise, select
it with TIS:

```sh
swift - <<'SWIFT'
import Carbon
import Foundation

let target = "com.apple.inputmethod.Korean.2SetKorean"
let list = TISCreateInputSourceList(nil, false).takeRetainedValue() as! [TISInputSource]

for source in list {
    if let unmanaged = TISGetInputSourceProperty(source, kTISPropertyInputModeID) {
        let mode = Unmanaged<CFString>.fromOpaque(unmanaged).takeUnretainedValue() as String
        if mode == target {
            let status = TISSelectInputSource(source)
            print("selected \(mode), status=\(status)")
            exit(status == noErr ? 0 : 1)
        }
    }
}

print("target input source not found")
exit(2)
SWIFT
```

Expected output:

```text
selected com.apple.inputmethod.Korean.2SetKorean, status=0
```

You can also confirm the selected source:

```sh
defaults read com.apple.HIToolbox AppleSelectedInputSources
```

Look for:

```text
"Input Mode" = "com.apple.inputmethod.Korean.2SetKorean";
```

## 4. Type `한글` through key events

Make the sample frontmost again, then send the physical key codes for `gksrmf`,
which produces `한글` on Korean 2-Set.

```sh
osascript <<'APPLESCRIPT'
tell application "System Events"
  set frontmost of application process "ime_textbox" to true
  delay 0.2
  key code 5
  delay 0.05
  key code 40
  delay 0.05
  key code 1
  delay 0.05
  key code 15
  delay 0.05
  key code 46
  delay 0.05
  key code 3
end tell
APPLESCRIPT
```

The key-code sequence is:

```text
5=g, 40=k, 1=s, 15=r, 46=m, 3=f
```

## 5. Check the result

The terminal log should include `DeleteSurrounding`-driven replacements as the
IME combines jamo into syllables. A successful run looks like this (captured
from a real run; note that starting a new syllable re-affirms the previous one
with an extra commit/delete-surrounding pair before the new jamo commits --
this is real Korean 2-Set behavior, not a bug):

```text
ime_enabled
commit "ㅎ" -> text="ㅎ"
delete_surrounding before=3 after=0 -> text=""
commit "하" -> text="하"
delete_surrounding before=3 after=0 -> text=""
commit "한" -> text="한"
delete_surrounding before=3 after=0 -> text=""
commit "한" -> text="한"
commit "ㄱ" -> text="한ㄱ"
delete_surrounding before=3 after=0 -> text="한"
commit "그" -> text="한그"
delete_surrounding before=3 after=0 -> text="한"
commit "글" -> text="한글"
```

The window title should end with:

```text
text: "한글|"
```

You can read the title directly:

```sh
osascript -e 'tell application "System Events" to get name of window 1 of application process "ime_textbox"'
```

Expected output contains:

```text
IME textbox (...) | text: "한글|" | preedit: ""
```

## Failure signal

The original bug leaves decomposed jamo in the committed buffer, especially for
the first syllable after switching to Korean IME. A failing run may show text
like:

```text
ㅎㅏㄴ
```

or logs that commit jamo without the surrounding-text deletion/replacement path.

## 6. Space-confirm sequence (no lost or doubled space)

This checks the fix for the regression the protocol-signal rewrite addresses:
confirming a Korean syllable with Space must produce exactly one space, not
zero (lost) or two (duplicated).

With the sample frontmost and Korean 2-Set selected, type `s` (`한`, key codes
`5 40 1`) then Space (key code `49`):

```sh
osascript <<'APPLESCRIPT'
tell application "System Events"
  set frontmost of application process "ime_textbox" to true
  delay 0.2
  key code 5
  delay 0.05
  key code 40
  delay 0.05
  key code 1
  delay 0.3
  key code 49
end tell
APPLESCRIPT
```

Expected log tail:

```text
commit "한" -> text="한"
delete_surrounding before=3 after=0 -> text=""
commit "한" -> text="한"
key " " -> text="한 "
```

The space arrives as a forwarded `key` event (not a second `Ime::Commit`), and
the title must read `text: "한 |"`. A regression shows either `text: "한|"`
(space lost) or `text: "한  |"` / two `commit`/`key` entries for the same
space press (space duplicated).

This also composes correctly into a full word: typing `gksrmf` (`한글`) then
Space should end with `text: "한글 |"` and exactly one trailing `key " "`
entry after the last `commit "글"`.

## 7. Pinyin candidate selection (digit does not leak as text)

This checks that a digit used only to *select* an IME candidate is not also
inserted as a literal character once the candidate commits.

Requires a Chinese Pinyin input source (e.g. `com.apple.inputmethod.SCIM.ITABC`,
Simplified Chinese "Pinyin - Simplified") to already be enabled in System
Settings > Keyboard > Input Sources. Enabling it purely from a script was not
reliable in this environment (`TISEnableInputSource` succeeded but the
subsequent `TISSelectInputSource` returned `paramErr` (-50), which looks like
it requires the interactive "add input source" confirmation) -- add it once by
hand, then this section can be scripted like the others:

```sh
swift - <<'SWIFT'
import Carbon
import Foundation

let target = "com.apple.inputmethod.SCIM.ITABC"
let list = TISCreateInputSourceList(nil, false).takeRetainedValue() as! [TISInputSource]

for source in list {
    if let idPtr = TISGetInputSourceProperty(source, kTISPropertyInputSourceID) {
        let id = Unmanaged<CFString>.fromOpaque(idPtr).takeUnretainedValue() as String
        if id == target {
            let status = TISSelectInputSource(source)
            print("selected \(id), status=\(status)")
            exit(status == noErr ? 0 : 1)
        }
    }
}

print("target input source not found or not enabled")
exit(2)
SWIFT
```

With the sample frontmost, type `nihao` then `1` to pick the first candidate
(exact key codes depend on layout; using the physical QWERTY codes for
`n i h a o 1` works with the default US layout under Pinyin). Expected log:
a single `commit "你好"` (or whichever candidate was first) with **no**
additional `key "1"` or `commit "1"` entry, and the title's `text` field must
not contain a stray `1`.

If the input source cannot be enabled/selected in the current environment,
skip this section and note it as skipped in the test report -- do not attempt
to force it through `defaults write`, which can leave the system's real input
source list in an inconsistent state.

## Cleanup

Close the sample window, press Escape while the sample is focused, or stop the
terminal process with `Ctrl-C`.
