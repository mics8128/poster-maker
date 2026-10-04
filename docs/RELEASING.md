# 發布流程

版本以 `package.json` 為唯一來源，支援 `X.Y.Z` 與 `X.Y.Z-rc.N`。本專案繼續使用 Node 22、pnpm 10、stable Rust；macOS runner 為 `macos-15`，目標仍是 Apple Silicon。Windows 僅打包 NSIS。

## 1. 準備新版本

先確認 [Releases](https://github.com/mics8128/poster-maker/releases) 和遠端 tags，選未使用的 patch 版號。

```bash
git fetch origin --tags
git tag --sort=-version:refname
# 修改 package.json 的 version，例如 0.3.2
pnpm sync-version
pnpm check:version
pnpm test:scripts
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

`sync-version` 同步 `Cargo.toml`、`Cargo.lock` 的本專案版本，以及 Tauri 的版本／視窗標題；不解析或升級依賴。一起提交這些版本檔與 README 的最新版本號。

## 2. 先驗證，不發布

開 PR，等待 CI 的 macOS 和 Windows jobs 成功。CI 和 Release 共用 `build.yml`，會做：

- 發布腳本測試與版本一致性檢查
- Rust 格式與 `cargo test --locked`
- TypeScript／Vite 前端 build
- Apple Silicon DMG／Windows NSIS 打包
- macOS ad-hoc 簽章驗證
- 保存兩個 installer 為 Actions artifacts

也可以在 Actions 的 **CI** 頁面手動選分支執行。這條流程沒有 release 寫入權限，不建立 tag 或 GitHub Release，也不碰既有下載檔。請勿用 Release 的手動執行來做純建置驗證。

## 3. 發布新 tag

PR 審查、CI 通過並合併後，在確認過的 master commit 建立 tag：

```bash
git switch master
git pull --ff-only
pnpm check:version
node scripts/sync-version.mjs --check --tag=v0.3.2
git tag v0.3.2
git push origin v0.3.2
```

推送新 tag 會啟動 Release：

1. 確認 tag 已存在、版本一致，且沒有同名 Release
2. 記住 tag 的確切 commit SHA，以同一份程式碼打包兩個平台
3. 發布前再次確認 tag 沒移動，且同名 Release 仍不存在
4. 產生 `SHA256SUMS.txt`，上傳新 draft Release
5. 下載剛上傳的檔案並比對 SHA-256，全部通過後公開

手動 Release 的 version 欄位必須填入**已存在且尚未發布**的 tag；打包的是該 tag，不是 UI 選定分支的程式碼。它不會自動修改版本、升級依賴、建立 tag，或覆寫既有 Release。含 `-` 的版號會標記為 prerelease。

## 失敗與重試

- 建置失敗且還沒有 Release：修正後使用新 patch tag，或對相同未變動 tag 重跑失敗 jobs
- 權限、網路或 GitHub API 錯誤：流程停止，不把錯誤當成「Release 不存在」
- 已有同名 Release（包含 draft）：停止，不刪除或覆寫 assets
- 若上傳／校驗中斷而留下 draft：先檢查該次 run、tag SHA、draft 和下載檔；由維護者確認如何處理，不要自動刪除重建
- 不移動已發布 tag，不以新版二進位檔取代舊版下載檔；需要修正就升下一個 patch

## 發布後確認

核對 Release tag 指向預期 commit，頁面有 DMG、EXE 和 `SHA256SUMS.txt`，實際下載並驗證：

```bash
# macOS
shasum -a 256 -c SHA256SUMS.txt
# Linux
sha256sum --check SHA256SUMS.txt
```

保留舊版 Release 與 assets。macOS 仍採 ad-hoc 簽章、未 notarized；這套流程不新增憑證、簽章服務或 auto-updater。
