# T113S3 Pro mainline FEL 20260904-rc1

这是面向 DshanPi T113S3 Pro + Winbond W25N02KV SPI-NAND 的主线系统预发布
版本，包含 U-Boot 2026.07、Linux 6.18.8、Buildroot/UBIFS 根文件系统、纯主线
FEL RAM 安装器及完整可重建源码。

## 发布状态

**Prerelease / hardware verification incomplete.**

2026-09-04 实测任务 `mainline-1788508715752023200` 成功完成 FEL 启动、载荷
校验、SPL/U-Boot 写入和 SHA-256 回读，以及 `sys.ubi` 写入和 UBI 挂载。任务
在最终 `installer_verify_rootfs` 94% 阶段未输出完成标志，Lynx MCP 在 180 秒
后结束监视。因此本候选版本不得标记为硬件完整验证，也不替代仓库中
2026-08-25/2026-08-26 的已验证基线。发布归档在该测试后重新打包，FIT 时间戳
和 UBI 元数据使其与测试文件不完全相同；其精确哈希记录在
`manifests/release-candidate-20260904-rc1.sha256`，且该归档本身没有硬件验证。

## 发布资产

- `t113s3pro-mainline-fel-20260904-rc1-images.tar.zst`：FEL 启动文件、分块
  载荷和三个持久 NAND 镜像；
- `t113s3pro-mainline-fel-20260904-rc1-source.tar.zst`：本标签的完整仓库源码，
  不包含可再下载的构建目录；
- `SHA256SUMS`：两个归档的 SHA-256；
- GitHub 自动生成的 Source code 归档也对应同一个标签。

镜像内容、源码入口、Lynx MCP 与命令行烧录步骤，以及日志验收方法见
[`../images-and-fel-flashing.zh-CN.md`](../images-and-fel-flashing.zh-CN.md)。

## 本次新增

- 尝试改用 64 KiB 块回读 `boot` UBI 卷，以避开 BusyBox `dd bs=1` 的逐字节慢路径；
- 补充中文镜像、源码和 FEL 烧录说明；
- 记录 MCP 180 秒监视边界、主线 U-Boot 无 `efex` 及 `0xff` 填充警告语义；
- 明确 94% 只代表进入最终验证，不能当作烧录完成。

## 校验与构建

解压前先运行：

```sh
sha256sum -c SHA256SUMS
```

源码重建入口：

```sh
./scripts/one-click-build.sh
```

由于本候选版本的硬件最终验证尚未完成，生产或批量烧录应继续使用明确标记为
hardware-verified 的基线。
