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
- Sys Layer 型別安全：用 `Signal` / `PollTimeout` 擋住外層裸整數，stdin/stdout fd 收成 `sys.rs` 內部 private newtype。
- Performance benchmark harness：`--benchmark --frames ...` 結束時輸出 key=value 統計，包含 frame count、elapsed、FPS、總 bytes、平均/max frame bytes 與 MiB/s。
- Benchmark snapshot 文件：`BENCHMARKS.md` 記錄可重跑的本機性能樣本，README 保留方法與連結，不硬寫不可追溯的性能宣稱。
- CLI Option Spec 資料化：用 `OPTION_SPECS` 集中短選項、長選項與 arity，parser 不再分散維護 `match name` / `long_to_short` / `option_requires_value`。
- FrameSymbol / animation 語意型別：frame raw strings 收在 `animation.rs` 內部，renderer 透過 `FrameSymbol` / `frame_symbol()` 取得語意化 symbol，palette lookup 仍是 O(1) array index。

## 未實做方向

### 1. 錯誤語意整理

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

## 建議順序

1. app-level error enum，只有當 runtime / telnet / CLI 錯誤面真的需要統一語意時再做

## Review Checklist

每做完一刀至少確認：

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --release`
- 一般模式 smoke：`--frames 1 --no-title --no-clear --no-counter`
- telnet 模式 smoke：`--telnet --skip-intro --frames 1 --no-title --no-clear --no-counter`
- 相關路徑 smoke，例如 truecolor、crop、CLI error path
