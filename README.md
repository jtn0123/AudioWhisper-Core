# AudioWhisper Core

The text rules AudioWhisper applies between speech recognition and pasting, in Rust, shared by
the [Mac app](https://github.com/jtn0123/AudioWhisper) and the Android keyboard:

* **spoken punctuation**: "Hi comma how are you question mark" → "Hi, how are you?";
* **filler words**: "Um, so I was, uh, thinking" → "So I was thinking";
* **custom vocabulary**: spellings to enforce ("kubernetes" → "Kubernetes", "audio whisper" →
  "AudioWhisper") and phrases to rewrite ("post gress -> Postgres"), plus the recogniser prompt;
* **cursor context**: a space and the right capital for the text before the cursor.

They began as the Mac app's Swift (`TranscriptFormatter`, `CustomVocabulary`,
`CursorContextFormatter`) and keep its exact output. The Mac app runs the old Swift beside this
crate in a differential test, which also catches a macOS update changing the Unicode data.

## Layout

| Path | What |
|---|---|
| `crates/apple-text` | Swift's, Foundation's and ICU's text semantics, shared by the ports: `swift` reproduces Swift's and Foundation's string functions, and `tables` holds the Unicode data, generated from macOS, that they and the ICU matching run on. |
| `crates/transcript` | The rules. `formatter`, `vocabulary`, `phrase` and `cursor` are the ports; `icu` reproduces `NSRegularExpression`'s matching over fancy-regex. |
| `crates/awcore` | A C interface (`include/awcore.h`) for the Mac app's Swift, built as a static library. |
| `tools/build-macos.sh` | Builds a universal (arm64 + x86_64) `libawcore.a`. |
| `tools/unicode-tables.sh` | Regenerates `crates/apple-text/src/tables/*.rs` from macOS's ICU, Swift runtime and Foundation. |

## Matching Swift exactly

The Swift ran on Foundation and ICU (`NSRegularExpression`), whose behaviour differs from Rust's
std and the regex crate in ways real text can reach:

* A Swift `Character` is a grapheme cluster, `==` is canonical equivalence, and `isLetter` /
  `isNumber` use the Swift runtime's data (which calls the CJK numerals numeric).
* `CharacterSet.whitespaces` includes U+200B, and Foundation's `replacingOccurrences` skips a
  match that a following combining character attaches to, while Swift's `range(of:)` matches
  whole characters only.
* ICU matches a case-insensitive literal by **full** case folding (a term "STRASSE" rewrites
  "Straße"; "full ﬆop" is a full stop), and adds U+0345 to `[\p{L}]`. `phrase.rs` matches
  vocabulary phrases the way ICU's matcher does, as a regex can only list every spelling.
* With `anchorsMatchLines`, ICU's `^` and `$` know every line terminator.
* macOS's ICU is Unicode 17, newer than the regex crate's tables, so `\p{L}` and friends come from
  ICU's own data.

## Checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Code here never silences a warning (no `#[allow]` or `#[expect]`), and no file of ours exceeds 500
lines.
