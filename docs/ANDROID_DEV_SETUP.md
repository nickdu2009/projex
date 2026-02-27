# Android 开发环境配置（rustup + NDK + Tauri mobile）

> 本文档适用于从 Homebrew Rust 迁移到 rustup 并初始化 Tauri Android 工程的全流程。
> 完成后可直接运行 `npm run tauri android dev` / `npm run tauri android build`。

---

## 1. 安装 rustup（替换 Homebrew Rust）

macOS 机器默认 Homebrew Rust 只包含 host target，无法交叉编译 Android。
必须迁移到 **rustup** 管理的 Rust 工具链。

```bash
# 1) 卸载 Homebrew rust（保留 cargo-tauri，它在 .cargo/bin）
brew uninstall rust

# 2) 安装 rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# 选择 "1) Proceed with standard installation"

# 3) 重新打开终端或 source
source "$HOME/.cargo/env"

# 4) 验证
rustc --version   # 应显示 rustup 管理的版本（≥ 1.77）
cargo --version
```

---

## 2. 安装 Android Rust targets

```bash
rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  i686-linux-android \
  x86_64-linux-android
```

---

## 3. 确认 Android SDK / NDK

Android SDK 已检测到位于 `~/Library/Android/sdk/`，NDK 版本 `29.0.14206865`。

设置环境变量（建议写入 `~/.zshrc`）：

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"
export NDK_HOME="$ANDROID_HOME/ndk/29.0.14206865"
export PATH="$ANDROID_HOME/platform-tools:$PATH"
```

> 若 NDK 版本不同，请到 Android Studio → SDK Manager → SDK Tools → NDK (Side by side) 安装 NDK ≥ 26。
> Tauri 2 推荐 NDK 26+，因为它内置了 `libc++_shared.so`。

---

## 4. 安装 cargo-ndk（用于交叉编译时链接）

```bash
cargo install cargo-ndk
```

---

## 5. 初始化 Tauri Android 工程

```bash
cd /Users/duxiaobo/workspaces/nickdu/project-management
npm run tauri android init
```

这会生成 `src-tauri/gen/android/` 目录（Gradle 项目），包含：
- `app/src/main/AndroidManifest.xml`
- `app/build.gradle.kts`
- `app/src/main/java/com/nickdu/projex/MainActivity.kt`

---

## 6. 配置 WorkManager / Keystore 依赖

初始化完成后，按 `docs/ANDROID_SUPPORT.md` 中的实现计划，在
`src-tauri/gen/android/app/build.gradle.kts` 中添加依赖（已在代码里提供，见第 7 步）。

---

## 7. 开发运行（模拟器或真机）

```bash
# 启动 Android 模拟器（或连接真机开启 USB 调试）
$ANDROID_HOME/emulator/emulator -avd <your_avd_name> &

# 运行开发模式
npm run tauri android dev
```

---

## 8. 打包构建

```bash
npm run tauri android build
# APK 在 src-tauri/gen/android/app/build/outputs/apk/
```

### 8.1 本地签名 Release 构建

CI 通过环境变量注入签名参数（见 `publish.yml`）。本地如需构建签名 Release APK：

```bash
# 1. 生成 keystore（仅首次）
keytool -genkey -v \
  -keystore ~/projex-release.jks \
  -alias projex \
  -keyalg RSA -keysize 2048 -validity 10000

# 2. 设置环境变量
export ANDROID_KEYSTORE_PATH="$HOME/projex-release.jks"
export ANDROID_STORE_PASSWORD="<keystore 密码>"
export ANDROID_KEY_ALIAS="projex"
export ANDROID_KEY_PASSWORD="<key 密码>"

# 3. 构建
cd src-tauri/gen/android
./gradlew assembleRelease
# APK 在 app/build/outputs/apk/release/app-release.apk
```

> ⚠️ 务必备份 `projex-release.jks`，丢失后无法更新已发布版本。
> 不要将 keystore 文件提交到 Git 仓库。

### 8.2 CI 签名配置（GitHub Secrets）

在 `https://github.com/nickdu2009/projex/settings/secrets/actions` 配置：

| Secret | 值来源 |
|--------|--------|
| `ANDROID_KEYSTORE_BASE64` | `base64 -i ~/projex-release.jks \| pbcopy` |
| `ANDROID_STORE_PASSWORD` | 生成 keystore 时设置的密码 |
| `ANDROID_KEY_ALIAS` | `projex` |
| `ANDROID_KEY_PASSWORD` | 生成 key 时设置的密码 |

---

## 9. 常见问题

| 问题 | 解决 |
|------|------|
| `error: could not find native library for target aarch64-linux-android` | 确认 `NDK_HOME` 环境变量正确；cargo-ndk 已安装 |
| `Execution failed for task ':app:mergeDebugNativeLibs'` | NDK 版本不兼容，建议使用 NDK 26+ |
| `http://` endpoint 被拒绝 | 设计如此；Android MVP 仅允许 https，见 `docs/ANDROID_SUPPORT.md` |
| 同步不触发 | WorkManager 在 Doze 模式下最多延迟 15min；可用 `adb shell dumpsys jobscheduler` 检查任务状态 |
| 凭据丢失（卸载重装） | Keystore 绑定 app uid；重装后需重新填写 S3 凭据 |
