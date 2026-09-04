# T113S3 Pro mainline unified 20260904-rc1

本预发布版本把板级主线 Linux 工程、FEL/FES 主机工具 OpenixCLI，以及 RAM-only
loader 制作工具和所需 bin 合并到同一 `main` 分支，并同时发布：

- FEL RAM 安装镜像包；
- FES SPI-NAND 组件候选包；
- 可复现的 RAM-only FES loader；
- allwinner-loader 工具和全部配置/输入 bin；
- 包含上述三个代码库历史的统一源码归档；
- 中文镜像说明、源码位置和两套烧录步骤。

FEL 与 FES 当前都采用 1 MiB SPL、3 MiB U-Boot、1 MiB secure-storage、
251 MiB `sys`/UBI 布局。

验证边界：2026-08-26 的精确 v5 FES 集合已通过写入校验和冷启动。本 release
重新生成的 FES 候选包先后进行了四次人工恢复 FEL 后的独立验证，均由 Lynx
`lynx_start_flash` 执行。四次都在 RAM
loader/DRAM 初始化成功后未重新枚举为 FES，于写 NAND 前终止，
`committedBytes` 为空且 verify 未运行。因此该候选 FES 包用于复现和继续验证，
不能宣称为本轮硬件烧录成功。四次失败后均未自动重试。

loader 固定 SHA-256：

```text
26f4e5bc7a0e9ad77f3205c9a139a787b946c2812e6a521b7673a58e5b38f2b3
```

详细用法见 `docs/images-and-flashing.zh-CN.md`，本轮 MCP 结果见
`logs/fes-validation-20260904.jsonl`。
