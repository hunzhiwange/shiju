# VibeWindow 本地构建 Makefile
# 用法: make <target>
# 示例: make macos           # 本地构建 macOS 桌面 .app（含 CLI）
#       make release-all      # 构建所有平台
#       make release-macos    # 构建 macOS 发布包
#       make cli-local        # 本地 CLI 快速构建
#       make tui              # 启动 CLI TUI

SHELL := /bin/bash
.DELETE_ON_ERROR:

# 版本信息
VERSION := $(shell awk '/^version\s*=/ { gsub(/"/, "", $$3); print $$3; exit }' Cargo.toml)
GIT_SHA := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")

# 构建配置
PROFILE ?= release
OUT_DIR ?= dist
RELEASE_DIR ?= $(OUT_DIR)/release
RELEASE_KIND ?= all
RELEASE_TARGET ?= $(LOCAL_TARGET)
RELEASE_BUILD_FLAGS ?=
CARGO_HOME ?= $(HOME)/.cargo
CARGO_TARGET_DIR ?= $(CURDIR)/target
TEST_TIMEOUT ?= 600
COVERAGE_DIR ?= coverage
PACKAGE ?=
TEST_ARGS ?=
RELEASE_CHECK_CLEAN ?= 0
RELEASE_CHECK_DENY ?= 1
RELEASE_CHECK_DIST ?= 1
RELEASE_CHECK_PUBLISH ?= 0
RELEASE_CHECK_WASM ?= 1
TARGET_DIR := $(CARGO_TARGET_DIR)
LOCAL_TARGET := $(shell rustc -vV 2>/dev/null | awk '/^host:/ { print $$2 }')
CLOC_SOURCE_DIRS := $(sort $(wildcard crates/*))
CLOC_EXCLUDE_DIRS := target,node_modules,dist,coverage,.turbo,.vite,.next
CLOC_INCLUDE_EXT := rs,html,js,ts,tsx,jsx,mjs,cjs,css,scss,less
RELEASE_PUBLISH_PACKAGES := \
        vw-api-types \
        vw-shared \
        vw-config-types \
        vw-gateway-client \
        vw-acp \
        vw-provider-resolver \
        vw-agent \
        vw-cli \
        vw-webview \
        vw-desktop
export CARGO_HOME
export CARGO_TARGET_DIR

# 颜色输出
CYAN := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
RED := \033[31m
RESET := \033[0m

# 目标平台
MACOS_X64 := x86_64-apple-darwin
MACOS_ARM := aarch64-apple-darwin
LINUX_X64_GNU := x86_64-unknown-linux-gnu
LINUX_X64_MUSL := x86_64-unknown-linux-musl
LINUX_ARM64_GNU := aarch64-unknown-linux-gnu
LINUX_ARM64_MUSL := aarch64-unknown-linux-musl
WINDOWS_X64 := x86_64-pc-windows-msvc
WINDOWS_ARM := aarch64-pc-windows-msvc

# 伪目标
.PHONY: help \
        release-all release-local release-macos release-linux release-windows \
        release-check release-check-clean release-check-fetch release-check-version \
        release-check-clippy release-check-test release-check-wasm \
        release-check-dist-plan release-check-deny release-check-publish-dry-run \
        release-package-target \
        release-package-local release-cli-local release-acp-local release-desktop-local \
        release-package-macos-x64 release-package-macos-arm \
        release-package-linux-x64 release-package-linux-arm64 \
        release-package-windows-x64 release-package-windows-arm \
        release-macos-app release-macos-zip release-macos-dmg release-macos-packages \
        release-windows-zip release-windows-zip-arm \
        release-goreleaser-check release-goreleaser-snapshot \
        cli-all cli-macos cli-linux cli-windows cli-local \
	tui \
        desktop-check desktop-run desktop-release desktop-wasm \
        ensure-wasm-target \
        cross-install cross-check \
        clean distclean \
        info check-deps \
        macos windows app-macos-with-cli app-windows-with-cli \
        worktree-commit-merge worktree-sync-main worktree-reset-all \
        fmt-check clippy check-wasm check-all test unit-test unit-test-package coverage unit-coverage unit-coverage-package run agent \
        cloc

# 默认目标
.DEFAULT_GOAL := help

## ============================================================================
## 帮助信息
## ============================================================================

help: ## 显示帮助信息
	@echo "$(CYAN)VibeWindow 构建系统$(RESET)"
	@echo "版本: $(VERSION) (SHA: $(GIT_SHA))"
	@echo ""
	@echo "$(GREEN)使用方法:$(RESET) make [target]"
	@echo ""
	@echo "$(GREEN)发布构建 (所有平台):$(RESET)"
	@grep -E '^release-.*##' $(MAKEFILE_LIST) | sed 's/release-/  /' | column -t -s '##'
	@echo ""
	@echo "$(GREEN)CLI 构建:$(RESET)"
	@grep -E '^cli-.*##' $(MAKEFILE_LIST) | sed 's/cli-/  /' | column -t -s '##'
	@echo ""
	@echo "$(GREEN)TUI 启动:$(RESET)"
	@grep -E '^tui-.*##' $(MAKEFILE_LIST) | sed 's/^/  /' | column -t -s '##'
	@echo ""
	@echo "$(GREEN)Desktop 开发:$(RESET)"
	@grep -E '^desktop-.*##' $(MAKEFILE_LIST) | sed 's/desktop-/  /' | column -t -s '##'
	@echo ""
	@echo "$(GREEN)本地应用构建:$(RESET)"
	@grep -E '^(macos|windows|app-).*##' $(MAKEFILE_LIST) | column -t -s '##'
	@echo ""
	@echo "$(GREEN)工具与检查:$(RESET)"
	@grep -E '^(cross-|check-|info|clean|cloc).*##' $(MAKEFILE_LIST) | column -t -s '##'
	@echo ""
	@echo "$(GREEN)测试与覆盖率:$(RESET)"
	@grep -E '^(test|unit-|coverage).*##' $(MAKEFILE_LIST) | column -t -s '##'
	@echo ""
	@echo "$(GREEN)环境变量:$(RESET)"
	@echo "  PROFILE=$(PROFILE) OUT_DIR=$(OUT_DIR) RELEASE_DIR=$(RELEASE_DIR) PACKAGE=$(PACKAGE) TEST_ARGS=$(TEST_ARGS)"
	@echo "  RELEASE_KIND=$(RELEASE_KIND) RELEASE_TARGET=$(RELEASE_TARGET) RELEASE_BUILD_FLAGS=$(RELEASE_BUILD_FLAGS)"
	@echo "  RELEASE_CHECK_CLEAN=$(RELEASE_CHECK_CLEAN) RELEASE_CHECK_DENY=$(RELEASE_CHECK_DENY) RELEASE_CHECK_DIST=$(RELEASE_CHECK_DIST)"
	@echo "  RELEASE_CHECK_PUBLISH=$(RELEASE_CHECK_PUBLISH) RELEASE_CHECK_WASM=$(RELEASE_CHECK_WASM)"

info: ## 显示构建信息
	@echo "版本: $(VERSION)"
	@echo "Git SHA: $(GIT_SHA)"
	@echo "Profile: $(PROFILE)"
	@echo "CARGO_HOME: $(CARGO_HOME)"
	@echo "CARGO_TARGET_DIR: $(CARGO_TARGET_DIR)"
	@echo "输出目录: $(OUT_DIR)"
	@echo "发布目录: $(RELEASE_DIR)"
	@echo "目标目录: $(TARGET_DIR)"
	@echo "本机 target: $(LOCAL_TARGET)"

## ============================================================================
## 发布构建 - 所有平台
## ============================================================================

release-all: release-macos release-linux release-windows ## 构建 macOS/Linux/Windows 发布包

release-local: release-package-local ## 构建当前机器 target 的完整便携包

release-package-target: ## 构建指定 target/kind 的便携包: RELEASE_KIND=all RELEASE_TARGET=$(LOCAL_TARGET)
	./scripts/package-release.sh --kind $(RELEASE_KIND) --target $(RELEASE_TARGET) --profile $(PROFILE) --out-dir $(RELEASE_DIR)/$(RELEASE_TARGET) $(RELEASE_BUILD_FLAGS)

release-package-local: ## 构建当前机器 target 的完整便携包 (CLI + ACP + 桌面 helper)
	./scripts/package-release.sh --kind all --target $(LOCAL_TARGET) --profile $(PROFILE) --out-dir $(RELEASE_DIR)/$(LOCAL_TARGET)

release-cli-local: ## 构建当前机器 target 的 CLI 包
	./scripts/package-release.sh --kind cli --target $(LOCAL_TARGET) --profile $(PROFILE) --out-dir $(RELEASE_DIR)/$(LOCAL_TARGET)

release-acp-local: ## 构建当前机器 target 的 ACP 包
	./scripts/package-release.sh --kind acp --target $(LOCAL_TARGET) --profile $(PROFILE) --out-dir $(RELEASE_DIR)/$(LOCAL_TARGET)

release-desktop-local: ## 构建当前机器 target 的桌面裸二进制包
	./scripts/package-release.sh --kind desktop --target $(LOCAL_TARGET) --profile $(PROFILE) --out-dir $(RELEASE_DIR)/$(LOCAL_TARGET)

release-macos: release-package-macos-x64 release-package-macos-arm release-macos-packages ## 构建 macOS 发布包 (x64/arm64 便携包 + 本机 .app zip/dmg)

release-package-macos-x64: ## 构建 macOS x64 完整便携包
	./scripts/package-release.sh --kind all --target $(MACOS_X64) --profile $(PROFILE) --out-dir $(RELEASE_DIR)/$(MACOS_X64)

release-package-macos-arm: ## 构建 macOS ARM64 完整便携包
	./scripts/package-release.sh --kind all --target $(MACOS_ARM) --profile $(PROFILE) --out-dir $(RELEASE_DIR)/$(MACOS_ARM)

release-macos-app: ## 构建 macOS .app，包含 helper/CLI/ACP
	INCLUDE_CLI_IN_APP=1 INCLUDE_ACP_IN_APP=1 ./scripts/bundle_macos.sh

release-macos-zip: ## 构建 macOS .app zip 包，包含 helper/CLI/ACP
	INCLUDE_CLI_IN_APP=1 INCLUDE_ACP_IN_APP=1 PACKAGE_APP=1 OUT_DIR=$(RELEASE_DIR)/macos ./scripts/bundle_macos.sh

release-macos-dmg: ## 构建 macOS DMG，包含 helper/CLI/ACP
	INCLUDE_CLI_IN_APP=1 INCLUDE_ACP_IN_APP=1 CREATE_DMG=1 OUT_DIR=$(RELEASE_DIR)/macos ./scripts/bundle_macos.sh

release-macos-packages: ## 一次构建 macOS .app zip + DMG
	INCLUDE_CLI_IN_APP=1 INCLUDE_ACP_IN_APP=1 PACKAGE_APP=1 CREATE_DMG=1 OUT_DIR=$(RELEASE_DIR)/macos ./scripts/bundle_macos.sh

release-linux: release-package-linux-x64 release-package-linux-arm64 ## 构建 Linux GNU 完整便携包 (需要 cross + Docker)

release-package-linux-x64: ## 构建 Linux x64 GNU 完整便携包
	./scripts/package-release.sh --kind all --target $(LINUX_X64_GNU) --profile $(PROFILE) --out-dir $(RELEASE_DIR)/$(LINUX_X64_GNU) --use-cross

release-package-linux-arm64: ## 构建 Linux ARM64 GNU 完整便携包
	./scripts/package-release.sh --kind all --target $(LINUX_ARM64_GNU) --profile $(PROFILE) --out-dir $(RELEASE_DIR)/$(LINUX_ARM64_GNU) --use-cross

release-windows: release-package-windows-x64 release-package-windows-arm release-windows-zip ## 构建 Windows 发布包 (便携包 + x64 zip)

release-package-windows-x64: ## 构建 Windows x64 完整便携包
	./scripts/package-release.sh --kind all --target $(WINDOWS_X64) --profile $(PROFILE) --out-dir $(RELEASE_DIR)/$(WINDOWS_X64) --use-xwin

release-package-windows-arm: ## 构建 Windows ARM64 完整便携包
	./scripts/package-release.sh --kind all --target $(WINDOWS_ARM) --profile $(PROFILE) --out-dir $(RELEASE_DIR)/$(WINDOWS_ARM) --use-xwin

release-windows-zip: ## 构建 Windows x64 桌面 zip，包含 helper/CLI/ACP
	INCLUDE_CLI_IN_APP=1 INCLUDE_ACP_IN_APP=1 TARGET=$(WINDOWS_X64) PROFILE=$(PROFILE) OUT_DIR=$(RELEASE_DIR)/windows/$(WINDOWS_X64) sh ./scripts/bundle_windows.sh

release-windows-zip-arm: ## 构建 Windows ARM64 桌面 zip，包含 helper/CLI/ACP
	INCLUDE_CLI_IN_APP=1 INCLUDE_ACP_IN_APP=1 TARGET=$(WINDOWS_ARM) PROFILE=$(PROFILE) OUT_DIR=$(RELEASE_DIR)/windows/$(WINDOWS_ARM) sh ./scripts/bundle_windows.sh

release-goreleaser-check: ## 校验 GoReleaser 发布配置
	@command -v goreleaser >/dev/null || { echo "$(RED)Error: goreleaser not found.$(RESET)"; exit 1; }
	goreleaser check

release-goreleaser-snapshot: ## 用 GoReleaser 在本地生成 snapshot 发布产物
	@command -v goreleaser >/dev/null || { echo "$(RED)Error: goreleaser not found.$(RESET)"; exit 1; }
	goreleaser release --snapshot --clean --skip=publish

release-check: release-check-clean release-check-fetch release-check-version release-check-clippy release-check-test ## 发版前本地校验，尽量提前覆盖 CI 发布失败点
	@if [ "$(RELEASE_CHECK_WASM)" = "1" ]; then \
		$(MAKE) release-check-wasm; \
	else \
		echo "$(YELLOW)Skipping wasm release check (RELEASE_CHECK_WASM=0).$(RESET)"; \
	fi
	@if [ "$(RELEASE_CHECK_DIST)" = "1" ]; then \
		$(MAKE) release-check-dist-plan; \
	else \
		echo "$(YELLOW)Skipping cargo-dist release check (RELEASE_CHECK_DIST=0).$(RESET)"; \
	fi
	@if [ "$(RELEASE_CHECK_DENY)" = "1" ]; then \
		$(MAKE) release-check-deny; \
	else \
		echo "$(YELLOW)Skipping cargo-deny release check (RELEASE_CHECK_DENY=0).$(RESET)"; \
	fi
	@if [ "$(RELEASE_CHECK_PUBLISH)" = "1" ]; then \
		$(MAKE) release-check-publish-dry-run; \
	else \
		echo "$(YELLOW)Skipping cargo publish dry-run (RELEASE_CHECK_PUBLISH=0).$(RESET)"; \
	fi
	@echo "$(GREEN)Release check passed.$(RESET)"

release-check-clean: ## 可选检查 git 工作区是否干净: RELEASE_CHECK_CLEAN=1
	@if [ "$(RELEASE_CHECK_CLEAN)" = "1" ]; then \
		echo "$(CYAN)Checking git worktree cleanliness...$(RESET)"; \
		git diff --quiet --ignore-submodules -- && git diff --cached --quiet --ignore-submodules -- || { \
			echo "$(RED)Error: git worktree has uncommitted changes.$(RESET)"; \
			exit 1; \
		}; \
	else \
		echo "$(YELLOW)Skipping clean worktree check (RELEASE_CHECK_CLEAN=0).$(RESET)"; \
	fi

release-check-fetch: ## 校验 Cargo.lock 可解析且依赖能按锁文件获取
	@echo "$(CYAN)Checking locked dependency resolution...$(RESET)"
	cargo fetch --locked

release-check-version: ## 校验 workspace crate 版本保持一致
	@echo "$(CYAN)Checking workspace package versions...$(RESET)"
	@cargo metadata --no-deps --format-version 1 | perl -0MJSON::PP -e 'my $$m = decode_json(<>); my %versions; for my $$p (@{$$m->{packages}}) { push @{$$versions{$$p->{version}}}, $$p->{name}; } if (keys(%versions) != 1) { for my $$version (sort keys %versions) { print STDERR "$$version: ", join(", ", sort @{$$versions{$$version}}), "\n"; } exit 1; } my ($$version) = keys %versions; print "workspace version: $$version\n";'

release-check-clippy: ## 对齐 CI clippy: 所有 target + 所有 feature + warnings deny
	cargo clippy --all-targets --all-features -- -D warnings

release-check-test: ## 对齐 CI 测试: workspace lib/bin/tests
	mkdir -p target/test-home
	CARGO_HOME="$${CARGO_HOME:-$$HOME/.cargo}" RUSTUP_HOME="$${RUSTUP_HOME:-$$HOME/.rustup}" HOME="$(CURDIR)/target/test-home" RUST_TEST_THREADS=1 perl -e 'alarm shift; exec @ARGV' $(TEST_TIMEOUT) cargo test --workspace --lib --bins --tests

release-check-wasm: ensure-wasm-target ## 对齐 CI wasm 构建
	cargo build -p vw-desktop --target wasm32-unknown-unknown --bin shi-ju

release-check-dist-plan: ## 校验 cargo-dist 发布矩阵和配置
	@command -v dist >/dev/null || { echo "$(RED)Error: dist not found. Install cargo-dist first.$(RESET)"; exit 1; }
	@mkdir -p "$(TARGET_DIR)/release-check"
	dist plan --output-format=json > "$(TARGET_DIR)/release-check/dist-plan.json"
	@echo "$(GREEN)Wrote $(TARGET_DIR)/release-check/dist-plan.json$(RESET)"

release-check-deny: ## 校验依赖安全公告、许可证与来源策略
	@command -v cargo-deny >/dev/null || { echo "$(RED)Error: cargo-deny not found. Install it with: cargo install cargo-deny --locked$(RESET)"; exit 1; }
	cargo deny check

release-check-publish-dry-run: ## 可选校验 crates.io 打包流程: RELEASE_CHECK_PUBLISH=1
	@for package in $(RELEASE_PUBLISH_PACKAGES); do \
		echo "$(CYAN)Checking cargo publish dry-run for $$package...$(RESET)"; \
		cargo publish -p "$$package" --dry-run --locked; \
	done

## ============================================================================
## CLI 构建 - macOS
## ============================================================================

cli-macos: cli-macos-local ## 构建 macOS CLI (本地架构)

cli-macos-x64: ## 构建 macOS x64 CLI
	@echo "$(CYAN)Building CLI for $(MACOS_X64)...$(RESET)"
	./scripts/build-cli.sh --target $(MACOS_X64) --profile $(PROFILE) --out-dir $(OUT_DIR)/cli/$(MACOS_X64)

cli-macos-arm: ## 构建 macOS ARM64 CLI
	@echo "$(CYAN)Building CLI for $(MACOS_ARM)...$(RESET)"
	./scripts/build-cli.sh --target $(MACOS_ARM) --profile $(PROFILE) --out-dir $(OUT_DIR)/cli/$(MACOS_ARM)

cli-macos-local: ## 构建 macOS CLI (本地架构，无交叉编译)
	@echo "$(CYAN)Building CLI for local macOS...$(RESET)"
	MODE=cli PROFILE=$(PROFILE) OUT_DIR=$(OUT_DIR)/cli/local ./scripts/bundle_macos.sh

## ============================================================================
## CLI 构建 - Linux (需要 cross)
## ============================================================================

cli-linux: cli-linux-x64 ## 构建 Linux x64 CLI

cli-linux-x64: ## 构建 Linux x64 GNU CLI
	@echo "$(CYAN)Building CLI for $(LINUX_X64_GNU)...$(RESET)"
	./scripts/build-cli.sh --target $(LINUX_X64_GNU) --profile $(PROFILE) --out-dir $(OUT_DIR)/cli/$(LINUX_X64_GNU) --use-cross

cli-linux-x64-musl: ## 构建 Linux x64 MUSL CLI
	@echo "$(CYAN)Building CLI for $(LINUX_X64_MUSL)...$(RESET)"
	./scripts/build-cli.sh --target $(LINUX_X64_MUSL) --profile $(PROFILE) --out-dir $(OUT_DIR)/cli/$(LINUX_X64_MUSL) --use-cross

cli-linux-arm64: ## 构建 Linux ARM64 GNU CLI
	@echo "$(CYAN)Building CLI for $(LINUX_ARM64_GNU)...$(RESET)"
	./scripts/build-cli.sh --target $(LINUX_ARM64_GNU) --profile $(PROFILE) --out-dir $(OUT_DIR)/cli/$(LINUX_ARM64_GNU) --use-cross

cli-linux-arm64-musl: ## 构建 Linux ARM64 MUSL CLI
	@echo "$(CYAN)Building CLI for $(LINUX_ARM64_MUSL)...$(RESET)"
	./scripts/build-cli.sh --target $(LINUX_ARM64_MUSL) --profile $(PROFILE) --out-dir $(OUT_DIR)/cli/$(LINUX_ARM64_MUSL) --use-cross

## ============================================================================
## CLI 构建 - Windows
## ============================================================================

cli-windows: cli-windows-x64 ## 构建 Windows x64 CLI

cli-windows-x64: ## 构建 Windows x64 CLI
	@echo "$(CYAN)Building CLI for $(WINDOWS_X64)...$(RESET)"
	./scripts/build-cli.sh --target $(WINDOWS_X64) --profile $(PROFILE) --out-dir $(OUT_DIR)/cli/$(WINDOWS_X64)

## ============================================================================
## CLI 构建 - 本地快速构建
## ============================================================================

cli-local: ## 本地快速构建 CLI (无交叉编译)
	@echo "$(CYAN)Building CLI for local target...$(RESET)"
	cargo build -p vw-cli --$(PROFILE) --bin vibe-agent --all-features
	@mkdir -p $(OUT_DIR)/cli/local
	@if [ -f "$(TARGET_DIR)/$(PROFILE)/vibe-agent$(EXE_SUFFIX)" ]; then \
		cp "$(TARGET_DIR)/$(PROFILE)/vibe-agent$(EXE_SUFFIX)" $(OUT_DIR)/cli/local/; \
	else \
		cp "$(TARGET_DIR)/debug/vibe-agent$(EXE_SUFFIX)" $(OUT_DIR)/cli/local/ 2>/dev/null || \
		cp "$(TARGET_DIR)/$(PROFILE)/vibe-agent" $(OUT_DIR)/cli/local/; \
	fi
	@echo "$(GREEN)Done: $(OUT_DIR)/cli/local/vibe-agent$(RESET)"

cli-all: cli-macos-x64 cli-macos-arm cli-linux-x64 cli-linux-arm64 cli-windows-x64 ## 构建所有平台 CLI

## ============================================================================
## TUI 启动
## ============================================================================

tui: ## 启动 CLI TUI
	cargo run --bin vibe-agent -- agent

## ============================================================================
## Desktop 开发
## ============================================================================

desktop-check: ## 检查 vw-desktop 是否可构建
	cargo check -p vw-desktop

desktop-run: ## 启动 vw-desktop 桌面应用
	cargo build -p vw-webview --bin vw-webview
	cargo run -p vw-desktop --bin shi-ju

desktop-release: ## 以 release 模式构建 vw-desktop
	cargo build -p vw-webview --release --bin vw-webview
	cargo build -p vw-desktop --release --bin shi-ju

ensure-wasm-target:
	@if ! rustup target list --installed | grep -qx 'wasm32-unknown-unknown'; then \
		echo "$(CYAN)Installing Rust target wasm32-unknown-unknown...$(RESET)"; \
		rustup target add wasm32-unknown-unknown; \
	fi

desktop-wasm: ensure-wasm-target ## 启动 vw-desktop wasm 开发服务 (trunk)
	cd crates/vw-desktop && env -u NO_COLOR trunk serve --color never --port 8199 index.html

## ============================================================================
## 本地应用构建 (原有目标)
## ============================================================================

macos: ## 本地构建 macOS 桌面 .app（含 CLI，不含 ACP/zip/DMG）
	INCLUDE_CLI_IN_APP=1 INCLUDE_CLI_COMPAT_IN_APP=0 INCLUDE_ACP_IN_APP=0 PACKAGE_APP=0 CREATE_DMG=0 ./scripts/bundle_macos.sh

windows: ## 本地构建 Windows 应用包
	sh ./scripts/bundle_windows.sh

app-macos-with-cli: macos ## 兼容别名：同 make macos

app-windows-with-cli: ## 本地构建 Windows 应用包，并将 CLI 一并放入压缩包
	INCLUDE_CLI_IN_APP=1 sh ./scripts/bundle_windows.sh

## ============================================================================
## Cross 编译工具安装
## ============================================================================

cross-install: ## 安装 cross 交叉编译工具
	@echo "$(CYAN)Installing cross...$(RESET)"
	cargo install cross --locked
	@echo "$(GREEN)cross installed successfully$(RESET)"
	@echo "$(YELLOW)Note: cross requires Docker to be running$(RESET)"

cross-check: ## 检查 cross 是否可用
	@command -v cross >/dev/null 2>&1 || { echo "$(RED)Error: cross not found. Run 'make cross-install' first.$(RESET)"; exit 1; }
	@docker info >/dev/null 2>&1 || { echo "$(RED)Error: Docker not running. Start Docker first.$(RESET)"; exit 1; }
	@echo "$(GREEN)cross and Docker are ready$(RESET)"

check-deps: ## 检查构建依赖
	@echo "$(CYAN)Checking build dependencies...$(RESET)"
	@echo -n "rustc: "; rustc --version 2>/dev/null || echo "$(RED)not found$(RESET)"
	@echo -n "cargo: "; cargo --version 2>/dev/null || echo "$(RED)not found$(RESET)"
	@echo -n "cross: "; cross --version 2>/dev/null || echo "$(YELLOW)not installed$(RESET)"
	@echo -n "cargo-xwin: "; cargo xwin --version 2>/dev/null || echo "$(YELLOW)not installed$(RESET)"
	@echo -n "goreleaser: "; command -v goreleaser >/dev/null 2>&1 && goreleaser --version | head -1 || echo "$(YELLOW)not installed$(RESET)"
	@echo -n "Docker: "; docker --version 2>/dev/null || echo "$(YELLOW)not installed$(RESET)"
	@echo -n "zip: "; zip --version 2>&1 | head -1 || echo "$(RED)not found$(RESET)"
	@echo -n "hdiutil (macOS DMG): "; hdiutil help >/dev/null 2>&1 && echo "available" || echo "$(YELLOW)not available$(RESET)"
	@echo -n "codesign (macOS): "; codesign --version 2>/dev/null || echo "$(YELLOW)not available$(RESET)"

cloc: ## 用 cloc 按语言统计 crates 各库源代码行数
	@command -v cloc >/dev/null || { echo "$(RED)cloc is required. Install it first.$(RESET)"; exit 1; }
	@for crate in $(CLOC_SOURCE_DIRS); do \
		if [ ! -f "$$crate/Cargo.toml" ]; then \
			continue; \
		fi; \
		echo ""; \
		echo "$(CYAN)== $$crate ==$(RESET)"; \
		cloc "$$crate" \
			--exclude-dir="$(CLOC_EXCLUDE_DIRS)" \
			--include-ext="$(CLOC_INCLUDE_EXT)" \
			--skip-uniqueness \
			--timeout 0 \
			--quiet; \
	done

## ============================================================================
## 清理
## ============================================================================

clean: ## 清理构建产物
	cargo clean
	rm -rf $(OUT_DIR)

distclean: clean ## 清理所有生成文件（包括缓存）
	rm -rf $(TARGET_DIR)
	rm -rf .cargo/registry
	rm -rf .cargo/git

## ============================================================================
## Worktree 管理
## ============================================================================

worktree-commit-merge:
	sh ./scripts/worktree_commit_merge.sh

worktree-sync-main:
	sh ./scripts/worktree_sync_main.sh

worktree-reset-all:
	sh ./scripts/worktree_reset_all.sh

## ============================================================================
## 代码质量检查
## ============================================================================

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

check-wasm: ensure-wasm-target
	cargo check --target wasm32-unknown-unknown

check-all:
	cargo check --all-features

test:
	mkdir -p target/test-home
	CARGO_HOME="$${CARGO_HOME:-$$HOME/.cargo}" RUSTUP_HOME="$${RUSTUP_HOME:-$$HOME/.rustup}" HOME="$(CURDIR)/target/test-home" RUST_TEST_THREADS=1 perl -e 'alarm shift; exec @ARGV' $(TEST_TIMEOUT) cargo test --workspace --lib --bins --tests

unit-test: ## 运行整个 workspace 的单元测试
	perl -e 'alarm shift; exec @ARGV' $(TEST_TIMEOUT) cargo test --workspace --lib --bins $(TEST_ARGS)

unit-test-package: ## 运行单个 package 的单元测试: make unit-test-package PACKAGE=vw-acp
	@test -n "$(PACKAGE)" || { echo "$(RED)Error: PACKAGE is required, e.g. make unit-test-package PACKAGE=vw-acp$(RESET)"; exit 1; }
	perl -e 'alarm shift; exec @ARGV' $(TEST_TIMEOUT) cargo test -p $(PACKAGE) --lib --bins $(TEST_ARGS)

coverage: unit-coverage ## 生成整个 workspace 的单元测试覆盖率报告

unit-coverage: ## 生成整个 workspace 的单元测试覆盖率报告
	TEST_TIMEOUT="$(TEST_TIMEOUT)" ./scripts/unit-coverage.sh --output-dir "$(COVERAGE_DIR)/workspace" $(TEST_ARGS)

unit-coverage-package: ## 生成单个 package 的单元测试覆盖率报告: make unit-coverage-package PACKAGE=vw-acp
	@test -n "$(PACKAGE)" || { echo "$(RED)Error: PACKAGE is required, e.g. make unit-coverage-package PACKAGE=vw-acp$(RESET)"; exit 1; }
	TEST_TIMEOUT="$(TEST_TIMEOUT)" ./scripts/unit-coverage.sh --package "$(PACKAGE)" --output-dir "$(COVERAGE_DIR)/$(PACKAGE)" $(TEST_ARGS)

run:
	cargo run --bin vibe-agent

agent:
	cargo run --bin vibe-agent
