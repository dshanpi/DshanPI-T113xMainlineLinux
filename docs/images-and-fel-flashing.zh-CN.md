# T113S3 Pro 纯主线 FEL 镜像与烧录说明

本文只描述 `feat/t113s3pro-mainline` 分支的纯主线 FEL RAM 安装器。它不使用
Tina/IMAGEWTY loader，也不进入 FES 服务，不能与
`feat/fes-nand-components` 分支的 FES 组件烧录包混用。

## 镜像是什么

release 中的硬件验证镜像归档为：

```text
t113s3pro-mainline-fel-recovery-hardware-verified-20260825.tar.zst
```

解压后的 `artifacts/pure-mainline-fel-persistent-final/` 目录包含 FEL 启动文件：

| 文件 | FEL 装载地址 | 用途 |
| --- | ---: | --- |
| `fel-sunxi-spl.bin` | `0x00020000` | 在 SRAM 初始化 128 MiB DRAM，然后返回 BootROM FEL |
| `fel-u-boot.bin` | `0x42e00000` | 在 DRAM 运行的 mainline U-Boot 2026.07 |
| `fel-installer.itb` | `0x44000000` | Linux 6.18.8、设备树和自包含 initramfs 安装器 |
| `fel-payload.part-*` | 从 `0x44800000` 连续装载 | 分块传入 RAM 的 NAND 安装载荷 |

`fel-payload.tar.gz` 中真正持久写入 SPI-NAND 的文件有四个：

| 文件 | NAND 目标 | 内容 |
| --- | --- | --- |
| `spl-redundant.bin` | `mtd0`，1 MiB | 八份 mainline eGON SPL |
| `uboot-redundant.bin` | `mtd1`，4 MiB | mainline U-Boot proper 和 `0xff` 填充 |
| `boot.itb` | `mtd3`，8 MiB raw boot 分区 | Linux 内核、设备树和 SHA-256 |
| `sys.ubi` | `mtd4`，242 MiB | 含 `rootfs` UBIFS 卷的 UBI 镜像 |

纯 FEL 分支的固定布局为：

| 区域 | 偏移 | 大小 |
| --- | ---: | ---: |
| `spl` | `0x00000000` | 1 MiB |
| `uboot` | `0x00100000` | 4 MiB |
| `secure-storage` | `0x00500000` | 1 MiB |
| `boot` | `0x00600000` | 8 MiB |
| `sys` | `0x00e00000` | 242 MiB |

它不同于 FES 分支的 `1 MiB SPL + 3 MiB U-Boot + 1 MiB secure-storage +
251 MiB sys/UBI` 布局。两个分支的镜像绝对不能交叉使用。

## 镜像源码

镜像源码位于本仓库 `feat/t113s3pro-mainline` 分支：

- `configs/dshanpi_t113s3pro_nand_defconfig`：Buildroot 配置；
- `board/dshanpi/t113s3pro/linux-dts/`：Linux 设备树；
- `board/dshanpi/t113s3pro/patches/`：Linux/U-Boot 补丁；
- `board/dshanpi/t113s3pro/make-mainline-fel-images.sh`：镜像与 FEL 载荷生成；
- `board/dshanpi/t113s3pro/installer-init`：板端 MTD/UBI 安装器；
- `manifests/sources.lock`：Buildroot、Linux 6.18.8 和 U-Boot 2026.07 的
  固定版本及校验值。

硬件验证镜像对应的源码基线是提交
`d1eedf7a97ae80043c1f2b72c30f649a24b2239f`。之后到本 release 标签之间没有
更改板级镜像生成代码，只增加了自动化、验证证据和本文档。

从源码构建：

```sh
git clone -b feat/t113s3pro-mainline \
  https://github.com/dshanpi/DshanPI-T113xMainlineLinux.git
cd DshanPI-T113xMainlineLinux
make all
```

## 使用 Lynx MCP 烧录

启动连接本机 Lynx MCP 的 Codex：

```sh
codex -c 'mcp_servers={"lynx-t113s3pro-ltslinux-sdk"={url="http://127.0.0.1:45110/mcp"}}'
```

烧录步骤：

1. 连接 UART3 PB6/PB7，波特率 115200。
2. 通过按键/启动脚位让板子物理进入 BootROM FEL。
3. 使用 `lynx_workbench_status`、`lynx_flash_capabilities` 和
   `lynx_scan_flash_devices` 确认只有一个目标 FEL 设备。
4. 使用 `lynx_download_firmware` 上传 `fel-sunxi-spl.bin`、
   `fel-u-boot.bin`、`fel-installer.itb` 和全部 `fel-payload.part-*`；每项
   传输任务及 SHA-256 都必须通过。
5. 调用 `lynx_allwinner_boot_mainline`，按上表地址装载，入口地址为
   `0x42e00000`。
6. 持续通过 `lynx_serial_read` 观察板端 Linux 安装器。FEL 传输完成只代表
   RAM 交接，不能代表 NAND 已烧好。
7. 看到 `installer_complete`、安装器重启后的登录提示后，再进行至少一秒的
   完全断电冷启动；冷启动必须从 `Trying to boot from sunxi SPI` 到达
   `t113s3pro-mainline login:`。

当前 mainline U-Boot 没有厂商 `efex` 命令。从已启动系统再次烧录时，必须物理
进入 FEL，不能在 U-Boot 输入 `efex`。

## 使用仓库脚本烧录

```sh
OPENIXCLI_BIN=../OpenixCLI/target/release/openixcli \
  ./scripts/flash-and-monitor.sh \
  ./out/t113s3pro-mainline-fel auto /dev/ttyACM0 300
```

仅执行 FEL RAM 交接、不做 UART 验收：

```sh
OPENIXCLI_BIN=../OpenixCLI/target/release/openixcli \
  ./scripts/flash-mainline-fel.sh \
  ./out/t113s3pro-mainline-fel auto
```

## 日志中真正的烧写过程

`Starting kernel ...` 之前是内存启动过程。真正写 NAND 从以下标志开始：

```text
LYNX_PROGRESS phase=installer_write_spl progress=62 partition=spl
LYNX_PROGRESS phase=installer_write_uboot progress=67 partition=uboot
LYNX_PROGRESS phase=installer_write_boot progress=72 partition=boot
LYNX_PROGRESS phase=installer_format_ubi progress=78 partition=sys
```

前三步分别擦除/写入 `mtd0`、`mtd1`、`mtd3`，随后都必须出现 readback
SHA-256 OK。`ubiformat ... flashing eraseblock ... 100%` 把 `sys.ubi` 写入
`mtd4`。最后安装器挂载 `ubi0:rootfs` 并检查 `/sbin/init`。

`nandwrite` 关于全 `0xff` 块可能被视为空块的提示来自镜像填充区，不等于写入
失败；必须以后续回读 SHA-256 为准。

完整成功标志是：

```text
LYNX_PROGRESS phase=installer_complete progress=100 partition=none
MAINLINE INSTALL COMPLETE - rebooting
```

MCP 的监视超时不证明板端已经停止。超时后只能继续被动读取 UART；不得自动
重试或立即断电。没有完成标志、登录提示和独立冷启动证据，就不能宣称成功。

## 已验证范围

本 release 复用 2026-08-25 保存的精确硬件验证归档。该归档在 DshanPi
T113S3 Pro、128 MiB DRAM、Winbond W25N02KV 256 MiB SPI-NAND 上完成安装、
回读验证、重启以及断电冷启动。它仍是开发/恢复方案，不代表任意 NAND 或量产
坏块分布已经获得资格认证。
