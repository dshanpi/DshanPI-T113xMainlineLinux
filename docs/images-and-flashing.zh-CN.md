# T113S3 Pro 镜像、源码与 FEL/FES 烧录说明

本文对应统一后的 `main` 分支。板级源码、OpenixCLI 和 Allwinner RAM loader
工具都在同一个仓库中；当前两种烧录方式使用同一套 256 MiB SPI-NAND 布局，
不能与历史纯 FEL release 的 1/4/1/8/242 MiB 镜像混用。

## 当前镜像是什么

主线系统由 U-Boot 2026.07、Linux 6.18.8 和 Buildroot/UBIFS 组成。持久化布局为：

| 区域 | 偏移 | 大小 | 内容 |
| --- | ---: | ---: | --- |
| `spl` | `0x00000000` | 1 MiB | 八份 mainline eGON SPL/Boot0 |
| `uboot` | `0x00100000` | 3 MiB | mainline U-Boot proper/Boot1 |
| `secure-storage` | `0x00400000` | 1 MiB | 保留区 |
| `sys` | `0x00500000` | 251 MiB | UBI：`boot`、`rootfs`、自动扩展空间 |

`boot` UBI 卷保存内核 FIT，`rootfs` 卷保存 UBIFS 根文件系统。冷启动链路是
BootROM -> mainline SPL -> mainline U-Boot -> `boot.itb` -> Linux ->
`ubi0:rootfs`。

构建后的 FEL RAM 安装目录为 `out/t113s3pro-mainline-fel/`，主要文件包括：

| 文件 | 作用 |
| --- | --- |
| `fel-sunxi-spl.bin` | 在 SRAM 初始化 DRAM，并返回 BootROM FEL |
| `fel-u-boot.bin` | 装入 DRAM 的 mainline U-Boot proper |
| `fel-installer.itb` | Linux、设备树和自包含 NAND 安装器 |
| `fel-payload.part-*` | 分块传入保留 RAM 的 SPL、U-Boot 和 UBI 载荷 |

FES 发布目录为 `out/t113s3pro-mainline-fes/`：

| 文件 | 作用 |
| --- | --- |
| `bootstrap-loader.img` | 只在 RAM 中运行，用于 DDR 初始化和进入 FES |
| `mainline-nand-components.img` | FES SPI-NAND 组件包 |
| `boot0-mainline.bin` | mainline SPL/Boot0 |
| `boot1-mainline.img` | mainline U-Boot/Boot1 |
| `boot.itb` | Linux FIT |
| `rootfs.ubifs` | UBIFS 根文件系统 |
| `manifest.json`、`SHA256SUMS` | 分区、角色、大小和 SHA-256 约束 |

loader 不是最终系统镜像，不写入任何 NAND 分区。

## 镜像和工具源码

- 板级配置、DTS、U-Boot/Linux 补丁：`board/dshanpi/t113s3pro/`；
- Buildroot defconfig：`configs/dshanpi_t113s3pro_nand_defconfig`；
- FEL 安装器和镜像生成：`board/dshanpi/t113s3pro/installer-init`、
  `board/dshanpi/t113s3pro/make-mainline-fel-images.sh`；
- FES 组件打包：`scripts/package-fes-components.sh`、
  `scripts/prepare-fes-bundle.py`；
- FEL/FES 主机工具源码：`tools/OpenixCLI/`；
- loader 制作工具、配置及所需 bin：`tools/allwinner-loader/`。其中
  `profiles/t113s3-ddr3-spinand-dshanpi-t113s3pro/input/` 包含六个制作输入，
  `loader.json` 固定每个输入的角色和 SHA-256；
- 所有外部源码版本和嵌入工具 tree：`manifests/sources.lock`。

## 构建

```sh
git clone https://github.com/dshanpi/DshanPI-T113xMainlineLinux.git
cd DshanPI-T113xMainlineLinux
./scripts/one-click-build.sh
```

OpenixCLI 和 loader 已在仓库内，不需要再克隆两个配套仓库。单独重建 loader：

```sh
make -C tools/allwinner-loader check dist
sha256sum tools/allwinner-loader/dist/t113s3-ddr3-spinand-dshanpi-t113s3pro-loader.bin
```

当前 loader 的固定 SHA-256 是
`26f4e5bc7a0e9ad77f3205c9a139a787b946c2812e6a521b7673a58e5b38f2b3`。

生成 FES 组件包还需要 Tina pack 工具目录：

```sh
TINA_SDK=/absolute/path/to/T113-tina5v1.2-sdk ./scripts/one-click-build.sh
```

脚本会自动采用仓库内生成的 loader。构建和预检都不会自动写板。

## FEL RAM 安装方式

1. 连接 UART3 PB6/PB7，115200 8N1，并让板子物理进入 FEL。
2. 构建 `tools/OpenixCLI/target/release/openixcli` 和 FEL 镜像。
3. 执行：

```sh
./scripts/flash-and-monitor.sh \
  ./out/t113s3pro-mainline-fel auto /dev/ttyACM0 300
```

主机依次把 SPL 装到 `0x00020000`、U-Boot 装到 `0x42e00000`、安装器 FIT
装到 `0x44000000`，载荷分块从 `0x44800000` 装入 RAM。真正写 NAND 从板端
日志的 `installer_write_spl`、`installer_write_uboot` 和
`installer_format_ubi` 开始。必须看到 SPL/U-Boot 回读 SHA-256 通过、
`installer_complete`、重启登录提示，并另外完成一次断电冷启动。

FEL 传输完成只表示 RAM 交接，不能单独作为烧录成功证据。失败后不要自动重试，
应重新物理进入 FEL。

## FES 组件烧录方式

FES 对 SPI-NAND 使用坏块感知的组件协议，不能使用 raw disk 模式。先做不接触
USB 的预检：

```sh
FES_BUNDLE=$PWD/out/t113s3pro-mainline-fes \
OPENIXCLI_BIN=$PWD/tools/OpenixCLI/target/release/openixcli \
make fes-preflight
```

板子进入 FEL 后，绑定明确的 USB bus/port，再执行破坏性全擦写：

```sh
DEVICE_LOCATION=libusb:BUS:PORT BUS=BUS PORT=PORT \
./scripts/flash-fes-nand.sh ./out/t113s3pro-mainline-fes
```

流程为：校验 manifest/组件 -> RAM loader 初始化 DDR -> 重新枚举 FES ->
查询 SPI-NAND -> 全擦除 -> 写 MBR -> 写并校验 `boot`/`rootfs` -> 关闭普通
分区访问 -> 写并校验 Boot1/Boot0 -> 保持停止状态。只有
`committedBytes`、所有组件校验成功以及进程正常退出才证明介质写入完成。
随后必须独立断电至少一秒并通过 UART 确认：

```text
Trying to boot from sunxi SPI
U-Boot 2026.07
ubi0: attached mtd3
VFS: Mounted root (ubifs filesystem)
t113s3pro-mainline login:
```

## Lynx MCP 对应步骤

使用 `lynx_workbench_status`、`lynx_flash_capabilities` 和
`lynx_scan_flash_devices` 锁定目标；用 `lynx_download_firmware` 上传组件包及
loader，并等待 SHA-256 校验成功；调用 `lynx_start_flash` 时选择
`mode=full_erase`、`verify=true`、`raw=false`、`postFlashAction=none`。持续查询
同一个 task ID，直到 `success`、`failed` 或 `cancelled`。FES 完成后再通过电源
控制器和独立 UART 做冷启动验证。

## 验证状态

2026-08-26 的固定 v5 FES 包已完成介质校验和独立冷启动，证据在
`logs/fes-hardware-validation-20260826.jsonl`、
`logs/fes-v5-cold-boot-20260826.log`。

2026-09-04 重新生成的候选组件包进行了三次相互独立、均由人工重新进入 FEL
开始的尝试。三次均由 Lynx `lynx_start_flash` 执行，并都在 loader 完成 DRAM
初始化后未重新枚举为 FES，于 8% 终止；没有 NAND committed bytes，校验未
运行，也没有自动重试。第二次失败后绑定 USB 上仍未发现设备，UART3 也没有
收到字节；第三次在启动前关闭 UART 监视句柄，结果仍相同，因而排除了 UART
监视句柄占用。这一可重复现象把问题范围
收窄到 RAM loader 执行后的 FES 启动/USB 重枚举阶段，而不是 MBR、分区内容或
NAND 写入阶段。结果记录在 `logs/fes-validation-20260904.jsonl`，不能称为烧录
成功，也不改变旧 v5 精确哈希集合的已验证状态。
