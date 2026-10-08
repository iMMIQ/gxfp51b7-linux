# GXFP51B7 Linux

[English](README.md) · [安装说明](docs/INSTALL.md) · [设计说明](docs/ARCHITECTURE.md) · [验证记录](docs/VALIDATION.md)

为 **华为 MACHC-WAX9 / ACPI GXFP51B7** 实现的实验性 Linux 指纹采集与 SDDM 登录支持。

已在真实机器上完成加密采集、独立手指匹配、PAM 对照验证及实际 SDDM 认证服务测试。项目通过专用采集后端和 PAM 模块，为这款机器提供 SDDM 登录支持。

## 当前状态

- 测试机器：MACHC-WAX9，BIOS 1.24，i7-10510U，SGX 已开启。
- 宿主：EndeavourOS，内核 7.2.9；登录管理器 SDDM 0.21。
- 原厂签名组件在本机 KVM 隔离环境内完成 BIOS 密钥解封和 TLS 通信，密钥保存在原厂 enclave 内。
- 食指新样本 6/6 通过，同一参与者的其他三根手指共 0/6 被接受。结果反映一位参与者的小样本验证；一般误认率、活体和防伪能力需要进一步评估。
- 实际 SDDM PAM 认证及账户检查通过；验证范围覆盖认证和账户阶段，完整桌面会话启动是后续检查项。

## 构建

准备编译器、PAM 开发头文件和 Python 3.11+，然后运行：

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -r requirements-tools.txt
make check PYTHON=python
```

离线检查覆盖编译、协议解析、图像解码、匹配计算和 PAM 配置生成。

## 安装与使用

安装使用仓库源码，以及自行准备的原厂组件、隔离虚拟机和通信身份。指纹模板在本机录入时生成。请依照[安装说明](docs/INSTALL.md)完成隔离环境准备、安装、录入和对照验证。新整理的安装及录入工具已通过离线检查，干净环境的端到端验证是下一步。每个新模板都需要单独验证；6/6、0/6 的数据对应原机器的原始模板。

接入后，在 SDDM 选择录入的账户，留空密码并提交登录，再轻触已录入的食指。识别失败或超时后可继续使用密码。

恢复密码登录分支：

```sh
sudo /usr/bin/python -I /usr/local/lib/gxfp51b7/manage.py disable
```

此命令只移除本项目标记的登录分支、禁用指纹配置并停止隔离环境，保留其他登录设置及私有资料。

## 发布内容与许可证

仓库提供源码、构建入口、离线测试、安装与恢复工具、设计说明及匿名汇总结果。指纹样本和模板、私钥、BIOS 容器、原厂组件及虚拟机镜像保存在本机私有目录。

用户态代码采用 **LGPL v3 或更新版本**。内核模块的独立许可范围见 [LICENSE.md](LICENSE.md)，外部参考与依赖见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
