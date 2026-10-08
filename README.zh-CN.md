# GXFP51B7 Linux

[English](README.md) · [安装说明](docs/INSTALL.md) · [fprintd 接入](docs/FPRINTD.md) · [验证记录](docs/VALIDATION.md)

为 **华为 MACHC-WAX9 / ACPI GXFP51B7** 提供 Linux 指纹登录。

Rust 负责加密采集、账户检查和录入流程，ChicagoHS 负责匹配，libfprint TOD
与标准 fprintd 负责模板管理和认证接口。SDDM 使用标准 pam_fprintd 完成指纹认证。
录入过程采集同一根手指的 12 个有效位置，并提供抬起、重新放置和位置调整提示。

## 适配配置

| 组件 | 配置 |
| --- | --- |
| 机器 / BIOS | 华为 MACHC-WAX9 / 1.24 |
| CPU | i7-10510U，SGX 已开启 |
| 传感器 / 固件 | ChicagoHS `0x2504` / `GF_9ELIBE_EC_19024` |
| 宿主系统 | EndeavourOS x86_64，内核 7.2.9 |
| 登录管理器 | SDDM 0.21，Arch `system-login` PAM 配置 |
| 隔离环境 | QEMU/KVM 11.1.2，Ubuntu 20.04，内核 5.4.0-216-generic |

## 工作方式

传感器通过 ACPI/EC 邮箱传输加密数据。原厂签名组件在本机 KVM 隔离环境内
解封通信密钥并验证采集数据，密钥保存在 enclave 内。宿主解码 80×64 图像，
检查接触质量并调用 ChicagoHS 匹配器。采集在内存中处理，校准数据和录入模板
保存在本机私有目录。

## 构建与检查

工程通过 `rust-toolchain.toml` 固定 **Rust 1.99.0**。准备 C 编译器、GLib/GIO、
libfprint TOD 开发头文件、pkg-config、PAM 开发头文件和 Python 3.11+，然后运行：

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -r requirements-tools.txt
cargo install cargo-machete --version 0.9.2 --locked
make check
```

检查涵盖构建、Rust 和原生算法测试、独立解码与质量对照、Rust/C/Python 格式化、
Clippy、Ruff、依赖使用情况和文档。内核模块使用 `make kernel` 构建。

## 安装与使用

依照[安装说明](docs/INSTALL.md)准备原厂组件、隔离虚拟机和通信身份，完成
宿主安装、空置传感器校准、fprintd 录入、同指与异指对照，再启用 SDDM。

```sh
fprintd-enroll -f right-index-finger YOUR_ACCOUNT
fprintd-verify -f right-index-finger YOUR_ACCOUNT
```

接入后，在 SDDM 选择录入账户，留空密码并提交登录，再轻触已录入的食指。
识别失败或超时后可继续使用密码。

恢复密码登录分支：

```sh
sudo /usr/local/lib/gxfp51b7/gxfp51b7 disable
```

## 发布内容与许可证

仓库提供源码、构建入口、离线测试、安装与恢复工具及汇总验证结果。
指纹样本和模板、私钥、BIOS 容器、原厂组件和虚拟机镜像保存在本机私有目录。

原创用户态代码采用 **LGPL v3 或更新版本**，内核模块提供双许可，ChicagoHS
上游源码保留其 LGPL v2.1 或更新版本许可。范围见 [LICENSE.md](LICENSE.md)，
来源与依赖见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
