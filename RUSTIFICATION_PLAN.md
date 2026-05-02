# Rustification Plan

這份文件用來追蹤 `refactor/rustify-core` 的後續方向：目標不是把程式「寫得像 Rust」而已，而是在不犧牲效能與相容性的前提下，逐步擺脫披著 Rust 外皮的 C-style 結構。

## 原則

- 先保行為，再改結構。每一刀都要能用測試或 smoke test 說明沒有改掉既有輸出語意。
- 小步 commit。每個 commit 聚焦一個概念邊界，方便 review 和回退。
- 不為抽象而抽象。只有當型別、trait 或封裝能降低錯誤機率、提高可測性，或保護 hot path 時才引入。
- 保留效能意識。render hot path 不做高成本 enum dispatch 或 allocation；新抽象要盡量是 zero-cost 或接近 zero-cost。
- unsafe 集中管理。Unix FFI 可以存在，但要收在清楚的邊界內，外層模組使用 safe API。

## 已完成

- 拆分 `main.rs`：將 CLI、render、runtime、sys、telnet、terminal 分成獨立模組。
- CLI 結構化：`parse_args` 回傳 `CliAction` / `CliError`，不再在 parser 內直接 exit。
- terminal 型別化：用 `TerminalType` 表達 terminal rendering mode。
- telnet subnegotiation 純化：TTYPE / NAWS parsing 可單元測試。
- deterministic frame rendering：`render_frame` 使用傳入的 elapsed seconds，避免直接讀 clock。
- render loop outcome：`render::run` 回傳 `RunOutcome`，不直接結束 process。
- Unix FFI 封裝：`unsafe` 呼叫集中到 `sys.rs`。
- telnet negotiation state：byte parser 和 negotiation state 拆成可測狀態物件。
- `main` 回傳 `ExitCode`：正常流程不再依賴 `process::exit()` 收尾。
- crop 型別化：`CropBounds` / `AxisCrop` 取代四個裸 crop 整數。
- palette 資料化：`PaletteEntry` 常數表取代大量 `palette.colors[b'x' as usize] = ...`。
- terminal size 型別化：`TerminalSize { width, height }` 取代裸 tuple / 分散整數。
- TerminalSession RAII：一般流程透過 scope-bound guard restore terminal，signal path 保留 raw write + `_exit`。
- FrameBuffer 抽象：render 和 intro 輸出透過薄 wrapper 集中處理 bytes、telnet newline、frame prefix 與 spacing。
- Renderer / RenderLoop 拆分：frame bytes 生成與 frame index、timing、frame limit 控制分離。
- animation frame consistency tests：測試所有 frame 的寬高固定，且所有 symbol 都可被 renderer palette 處理。
- Telnet I/O 抽象：negotiation loop 透過 `ByteSource` 讀 byte，stdin/poll 保留在 production path，測試可用 scripted input 驗完整 response。

## 未實做方向

### 1. FrameSymbol / animation 語意型別

動畫資料目前仍是 raw byte symbol。這很快，但語意偏 C。可以考慮輕量 newtype：

```rust
struct FrameSymbol(u8);
```

不建議急著改成完整 enum，因為 render hot path 會變囉嗦，也可能增加轉換成本。frame consistency tests 已先補上；後續只在 newtype 能清楚降低錯誤機率時再做。

完成標準：

- 若引入 `FrameSymbol`，palette lookup 仍是 O(1) 且不增加 allocation。
- 現有 frame consistency tests 繼續保護 frame data 正確性。

### 2. Sys Layer 型別安全

`sys.rs` 已集中 unsafe，但仍有裸 fd、signal number、timeout millis。可以引入小型 newtype / enum：

```rust
struct Fd(i32);
enum Signal {
    Interrupt,
    Pipe,
    WindowChanged,
}
```

完成標準：

- 外層模組不直接傳 magic fd / signal number。
- FFI wrapper 還是薄層，不把 libc 包成大型框架。
- unsafe block 仍集中在 `sys.rs`。

### 3. 錯誤語意整理

目前多數錯誤用 `io::Result` 即可，但如果 render、telnet、runtime 的錯誤面繼續擴大，可以新增 app-level error enum。

可能形式：

```rust
enum AppError {
    Io(io::Error),
    Cli(CliError),
    Telnet(io::Error),
}
```

完成標準：

- 只有當錯誤處理分支真的變複雜時才做。
- 不為了形式化而包一層錯誤型別。
- CLI error message 和 exit code 維持相容。

### 4. CLI Option Spec 資料化

CLI parsing 已經結構化，但 option dispatch 還是 match + char。可以資料化 option spec，不過目前優先級低，因為現有 match 可讀性仍高。

完成標準：

- 如果新增更多 option，再考慮資料表化。
- 不降低目前 error message 的精準度。
- 不引入外部 CLI crate，除非專案目標改變。

### 5. Performance / Benchmark 實測

目前重構主要改善結構與可測性，不是性能調校。後續若要主張高效能，應補可重複的測量方式。

可測項目：

- frame render throughput。
- ANSI / TrueColor / ASCII output size。
- benchmark mode 下 allocation 是否穩定。
- palette lookup 是否仍是 O(1)。

完成標準：

- benchmark 指令可重複執行。
- 記錄硬體 / build mode / flags。
- 不把 benchmark 結果硬寫進 README，除非有穩定方法重跑。

## 建議順序

1. sys layer typed fd / signal
2. performance benchmark harness
3. app-level error enum / CLI option spec，視後續複雜度決定
4. FrameSymbol / animation newtype，只有在能明確降低 frame data 風險時再做

## Review Checklist

每做完一刀至少確認：

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --release`
- 一般模式 smoke：`--frames 1 --no-title --no-clear --no-counter`
- telnet 模式 smoke：`--telnet --skip-intro --frames 1 --no-title --no-clear --no-counter`
- 相關路徑 smoke，例如 truecolor、crop、CLI error path
