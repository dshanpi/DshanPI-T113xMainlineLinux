# Third-party notice

Repository-authored source code and documentation are licensed under GPL-3.0 as stated in `LICENSE`.

The files under `tools/vendor/allwinner/` are unmodified Allwinner-compatible binary packing tools obtained from:

- <https://github.com/OpenNekoCloud/openixsuit-generic-img-flash-boot-image>
- source revision recorded in `tools/vendor/allwinner/SOURCE.json`

The runtime artifacts under `profiles/*/input/` originate from the named Allwinner/Tina SDK build recorded by each profile manifest. They are redistributed to make a specific RAM-only loader reproducible.

No upstream license was found embedded in these binary artifacts. Their SPDX status is therefore `LicenseRef-NOASSERTION`; they are not relicensed under this repository's GPL-3.0 license. Users and redistributors are responsible for confirming their rights to use these third-party artifacts.

