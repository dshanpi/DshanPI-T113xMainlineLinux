# Artifact status

## verified

`verified-hardware-artifacts.sha256` identifies the preserved bundle used by
installer tasks `mainline-1787655837814079629` and
`mainline-1787709324680503509`. Both completed at 100%, and both were followed
by a real power cycle that reached the UBIFS login prompt on the tested board.

## failed-do-not-use

`clean-build-20260825.sha256` identifies an internally consistent clean rebuild
that failed hardware task `mainline-1787708850567538011` at the RAM-installer
handoff. Do not flash those hashes again until source drift is resolved and a
new full hardware gate passes.

## experimental and recovery-only

Other historical bundles are not selected by this repository. They remain in
the private pre-clean archive with their original logs and hashes. Promotion
requires an explicit manifest plus install, readback and cold-boot evidence.
