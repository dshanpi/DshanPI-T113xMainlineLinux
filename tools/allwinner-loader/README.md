# Allwinner Loader Registry

全志芯片 RAM-only FEL/FES loader 的命名、来源、构建、校验和发布仓库。

本仓库中的 `loader.bin` 只用于以下临时引导链路：

```text
BootROM FEL -> FES1 初始化 DDR -> vendor U-Boot 在 RAM 运行
            -> USB Product/FES 重枚举 -> 外部工具写入目标系统镜像
```

loader 内的 FES、U-Boot 和板级配置不得作为目标系统写入 NAND/eMMC/NOR。最终落盘镜像与 loader 是两个独立输入。

## 文件命名标准

```text
<chip>-<memory-type>-<storage-type>-<product-name>-loader.bin
```

所有字段使用小写 ASCII kebab-case。当前标准枚举及 manifest 字段见 [Loader Standard v1](docs/loader-standard-v1.md)。例如：

```text
t113s3-ddr3-spinand-dshanpi-t113s3pro-loader.bin
```

文件名只能用于展示和初筛；自动化必须读取同名 `.manifest.json`，并校验二进制 SHA-256、六项 IMAGEWTY 条目 SHA-256、芯片 Device ID、内存和存储类型。

## 构建与校验

要求 Linux x86_64、Python 3.8+，以及运行原厂 32 位 `dragon` 所需的 i386 运行库：

```bash
sudo apt-get install libc6-i386 lib32stdc++6
make check
make dist
```

构建单个 profile：

```bash
python3 tools/allwinner_loader.py build \
  --manifest profiles/t113s3-ddr3-spinand-dshanpi-t113s3pro/loader.json \
  --output-dir dist
```

检查或解析 loader：

```bash
python3 tools/allwinner_loader.py verify \
  --manifest profiles/t113s3-ddr3-spinand-dshanpi-t113s3pro/loader.json \
  --image dist/t113s3-ddr3-spinand-dshanpi-t113s3pro-loader.bin

python3 tools/allwinner_loader.py inspect \
  dist/t113s3-ddr3-spinand-dshanpi-t113s3pro-loader.bin
```

## 在其他仓库调用

GitHub Actions 可直接调用仓库根目录的 composite action：

```yaml
- uses: dshanpi/allwinner-loader@v1.0.0
  with:
    manifest: path/to/loader.json
    output-dir: dist
```

本地 agent/Codex 集成使用 [allwinner-loader-builder Skill](skills/allwinner-loader-builder/SKILL.md)。仓库级强制约束见 [AGENTS.md](AGENTS.md)；下游仓库可复制 [Agent integration contract](integrations/AGENTS.allwinner-loader.md) 到自己的 `AGENTS.md`。

## Release

推送 `v*` tag 后，工作流会重新构建所有 profile，生成：

- `*-loader.bin`
- `*.manifest.json`
- `SHA256SUMS`
- `allwinner-loader-tools-<tag>.tar.gz`
- `allwinner-loader-sources-<tag>.tar.gz`

然后自动创建 GitHub Release 并上传这些文件。第三方原始工具和运行时 blob 的来源与许可边界见 [NOTICE](NOTICE.md)。
