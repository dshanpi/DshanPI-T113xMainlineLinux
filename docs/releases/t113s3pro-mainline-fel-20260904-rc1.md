# T113S3 Pro pure-mainline FEL 20260904-rc1

这是 `feat/t113s3pro-mainline` 分支的纯主线 FEL 恢复版，不包含 Tina loader、
FES 服务或 FES 对齐镜像。

发布资产包含：

- 2026-08-25 精确硬件验证的纯 FEL 镜像归档；
- 当前 FEL 标签的源码归档；
- 中文镜像组成、源码位置、MCP/命令行烧录步骤；
- 所有附件的 `SHA256SUMS`。

镜像使用纯 FEL 布局：1 MiB SPL、4 MiB U-Boot、1 MiB 保留区、8 MiB raw
`boot.itb`、242 MiB `sys/rootfs`。不得与 FES 分支的 1/3/1/251 MiB 镜像混用。

硬件验证镜像对应源码基线为
`d1eedf7a97ae80043c1f2b72c30f649a24b2239f`；后续分支提交仅增加主机自动化、
验证证据和发布文档，没有改变板级镜像生成源码。

详细说明见
[中文镜像与 FEL 烧录说明](https://github.com/dshanpi/DshanPI-T113xMainlineLinux/blob/t113s3pro-mainline-fel-20260904-rc1/docs/images-and-fel-flashing.zh-CN.md)。

本版本分类为 **prerelease / hardware-verified recovery-only**：已验证指定板卡和
NAND 的恢复流程，不作为跨 NAND 型号或量产坏块场景的资格声明。
