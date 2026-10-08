# GXFP51B7 Linux

[English](README.md) · [安装说明](docs/INSTALL.md) · [设计说明](docs/ARCHITECTURE.md) · [验证记录](docs/VALIDATION.md)

为 **华为 MACHC-WAX9 / ACPI GXFP51B7** 使用 Rust 实现的 Linux 指纹采集与 SDDM 登录支持。

项目提供 Rust 加密采集后端、指纹匹配算法和 PAM 模块，通过 SDDM 使用指纹登录。宿主运行编译后的 Rust 程序，数组处理、滤波、FFT 和插值使用 Rust 库。

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

指纹传感器通过 ACPI/EC 邮箱传输加密数据。原厂签名组件在本机 KVM 隔离环境内完成 BIOS 密钥解封和 TLS 通信，密钥保存在原厂 enclave 内。宿主解码 80×64 指纹图像，与本机录入的模板匹配，再由 PAM 模块完成登录认证。

## 构建

准备 Rust 1.88+、C 编译器、PAM 开发头文件，以及离线工具使用的 Python 3.11+，然后运行：

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -r requirements-tools.txt
make check PYTHON=python
```

离线检查覆盖编译、协议解析、图像解码、匹配计算和 PAM 配置生成。

## 安装与使用

安装使用仓库源码，以及自行准备的原厂组件、隔离虚拟机和通信身份。指纹模板在本机录入时生成。请依照[安装说明](docs/INSTALL.md)完成隔离环境准备、安装、录入和对照验证。

接入后，在 SDDM 选择录入的账户，留空密码并提交登录，再轻触已录入的食指。识别失败或超时后可继续使用密码。

恢复密码登录分支：

```sh
sudo /usr/local/lib/gxfp51b7/gxfp51b7 disable
```

此命令只移除本项目标记的登录分支、禁用指纹配置并停止隔离环境，保留其他登录设置及私有资料。

## 发布内容与许可证

仓库提供源码、构建入口、离线测试、安装与恢复工具、设计说明及匿名汇总结果。指纹样本和模板、私钥、BIOS 容器、原厂组件及虚拟机镜像保存在本机私有目录。

用户态代码采用 **LGPL v3 或更新版本**。内核模块的独立许可范围见 [LICENSE.md](LICENSE.md)，外部参考与依赖见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
