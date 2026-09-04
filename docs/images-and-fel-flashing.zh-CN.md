# T113S3 Pro 主线镜像与 FEL 烧录说明

本文说明 release 中每个镜像的用途，以及如何通过 Lynx MCP 或仓库脚本把主线
U-Boot 2026.07、Linux 6.18.8 和 Buildroot/UBIFS 系统安装到 DshanPi T113S3 Pro
的 256 MiB SPI-NAND。

## 安全边界

- 仅适用于已验证的 DshanPi T113S3 Pro、128 MiB DRAM 和 Winbond W25N02KV
  256 MiB SPI-NAND 组合。
- SPI-NAND 不是普通块设备，不能把某个整盘 `*.img` 用 `dd` 直接写入。
- 烧录会擦除 NAND 上现有的 SPL、U-Boot 和 `sys` UBI 系统。`secure-storage`
  分区保留。
- 一个任务失败后不得自动重试。必须让板子重新物理进入 FEL，再开始新任务。
- FEL 文件传输完成不等于 NAND 烧录成功；必须同时观察 UART 安装日志。

## 镜像是什么

构建输出目录是 `out/t113s3pro-mainline-fel/`。真正持久写入 NAND 的文件只有
下面三个：

| 文件 | NAND 目标 | 内容 |
| --- | --- | --- |
| `spl-redundant.bin` | `mtd0`，偏移 `0x00000000`，1 MiB | 每个 128 KiB 擦除块放置一份 mainline eGON SPL，共八份 |
| `uboot-redundant.bin` | `mtd1`，偏移 `0x00100000`，3 MiB | mainline U-Boot proper，其余空间填充为 `0xff` |
| `sys.ubi` | `mtd3`，偏移 `0x00500000`，251 MiB | UBI 镜像，含静态 `boot` 卷和自动扩展的 `rootfs` UBIFS 卷 |

`boot.itb` 包含 Linux 内核、设备树及 SHA-256，已经嵌入 `sys.ubi` 的 `boot`
卷；RAM 安装器不会再单独写一次 `boot.itb`。

下面这些文件只用于从 BootROM FEL 启动内存安装器，不是持久系统分区镜像：

| 文件 | 装载地址 | 用途 |
| --- | ---: | --- |
| `fel-sunxi-spl.bin` | `0x00020000` | 初始化 DRAM 后返回 BootROM FEL |
| `fel-u-boot.bin` | `0x42e00000` | 在 DRAM 中运行的 mainline U-Boot proper |
| `fel-installer.itb` | `0x44000000` | Linux 内核、设备树和自包含 initramfs 安装器 |
| `fel-payload.part-*` | 从 `0x44800000` 连续排列 | 分块传输的 SPL、U-Boot 和 `sys.ubi` 安装载荷 |

`FEL_SHA256SUMS` 是 FEL 传输文件校验表，`FEL_ARTIFACTS` 记录装载关系。release
根目录的 `SHA256SUMS` 用来校验所有发布资产。

## 镜像源码在哪里

所有可再分发源码和构建定义都在本仓库及 release 源码包中：

- `manifests/sources.lock` 固定 Buildroot、Linux、U-Boot 和 OpenixCLI 的版本、
  提交或官方归档 SHA-256；
- `configs/dshanpi_t113s3pro_nand_defconfig` 是 Buildroot 入口配置；
- `board/dshanpi/t113s3pro/linux-dts/` 包含 Linux 设备树；
- `board/dshanpi/t113s3pro/patches/` 包含 U-Boot/Linux 补丁；
- `board/dshanpi/t113s3pro/make-mainline-fel-images.sh` 生成上述持久镜像、UBI
  和 FEL 载荷；
- `board/dshanpi/t113s3pro/installer-init` 是板端 NAND 安装与回读校验程序；
- `scripts/one-click-build.sh` 完成锁定依赖、构建、打包与本地验证。

从源码完整重建：

```sh
git clone -b feat/fes-nand-components \
  https://github.com/dshanpi/DshanPI-T113xMainlineLinux.git
cd DshanPI-T113xMainlineLinux
./scripts/one-click-build.sh
```

## 使用 Lynx MCP 烧录

启动连接本机 Lynx 服务的 Codex：

```sh
codex -c 'mcp_servers={"lynx-t113s3pro-ltslinux-sdk"={url="http://127.0.0.1:45110/mcp"}}'
```

烧录流程如下：

1. 连接 UART3（PB6/PB7，115200）并让开发板物理进入 FEL。
2. 调用 `lynx_workbench_status`、`lynx_flash_capabilities` 和
   `lynx_scan_flash_devices`，确认目标是唯一在线的 Allwinner FEL 设备。
3. 对 `fel-sunxi-spl.bin`、`fel-u-boot.bin`、`fel-installer.itb` 和全部
   `fel-payload.part-*` 调用 `lynx_download_firmware`。每次下载任务都必须成功，
   且 MCP 返回的 SHA-256 必须与 `FEL_SHA256SUMS` 一致。
4. 调用 `lynx_allwinner_boot_mainline`，按上表地址排列文件，入口地址设为
   `0x42e00000`。该调用只是把安装环境交给板子；之后真正的 NAND 写入由板端
   Linux 完成。
5. 通过 `lynx_serial_read` 观察 UART，并按下一节的标志判断结果。不要只看 MCP
   传输百分比。
6. 只有看到 `installer_complete` 和安装后登录提示，才可执行一次至少一秒的
   完全断电冷启动。冷启动必须从 `Trying to boot from sunxi SPI` 一直走到
   `t113s3pro-mainline login:`。

主线 U-Boot 2026.07 当前没有 `efex` 命令。因此从已经启动的系统重新烧录时，
不能依靠在 U-Boot 输入 `efex` 返回 FEL，必须使用板卡的 FEL 按键/启动脚位并
重新上电或复位。

## 使用仓库脚本烧录

安装并构建锁定版本的 OpenixCLI 后，让板子进入 FEL并运行：

```sh
OPENIXCLI_BIN=../OpenixCLI/target/release/openixcli \
  ./scripts/flash-and-monitor.sh \
  ./out/t113s3pro-mainline-fel auto /dev/ttyACM0 300
```

如果只需要完成 FEL RAM 交接、不监视 UART：

```sh
OPENIXCLI_BIN=../OpenixCLI/target/release/openixcli \
  ./scripts/flash-mainline-fel.sh \
  ./out/t113s3pro-mainline-fel auto
```

第二种方式不能单独证明 NAND 已安装成功。

## 日志中哪一段才是烧写

以下日志之前都只是 FEL 传输和内存启动：

```text
Writing mainline SPL/U-Boot 2026.07 and Linux 6.18.8 layout...
```

真正的介质操作对应：

```text
LYNX_PROGRESS phase=installer_write_spl progress=62 partition=spl
Erasing 1024 Kibyte ...
```

擦除并写入 `mtd0`；随后：

```text
LYNX_PROGRESS phase=installer_write_uboot progress=67 partition=uboot
Erasing 3072 Kibyte ...
```

擦除并写入 `mtd1`。`/dev/mtd0 readback SHA-256 OK` 和
`/dev/mtd1 readback SHA-256 OK` 表示两者回读一致。最后：

```text
LYNX_PROGRESS phase=installer_format_ubi progress=78 partition=sys
ubiformat: flashing eraseblock ... 100 % complete
```

把 `sys.ubi` 写入 `mtd3`。`installer_verify_rootfs progress=94` 只是开始最终
回读、挂载和 `/sbin/init` 检查，并不表示整次安装已经成功。

`nandwrite` 输出 “blocks containing only 0xff ... may be incorrectly treated as
empty” 是对镜像填充区的提示，不是本次校验失败；后续 SHA-256 回读结果才是
判断 SPL/U-Boot 是否正确的依据。

## 成功判据和已知限制

完整成功必须同时满足：

- 所有主机侧和载荷 SHA-256 校验通过；
- SPL、U-Boot 和 `boot` 卷回读校验通过；
- `rootfs` 能以 UBIFS 挂载且存在可执行的 `/sbin/init`；
- UART 出现 `LYNX_PROGRESS phase=installer_complete progress=100`；
- 安装器重启和一次独立断电冷启动都到达登录提示。

2026-09-04 的任务 `mainline-1788508715752023200` 已写入 SPL、U-Boot 和
`sys.ubi`，前两个回读校验通过，UBI 识别出 `boot/rootfs` 且 NAND 报告零坏块；
但任务停在 `installer_verify_rootfs` 94%，180 秒内没有完成标志。该任务必须
记为超时/未完成，不能作为这个新 artifact 集的硬件成功证明。Lynx MCP 的固定
180 秒监视窗口也可能早于板端程序结束，因此超时后应继续只读 UART，不能自动
重烧或立即断电。
