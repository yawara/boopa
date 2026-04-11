## Why

`boopa` の authoritative DHCP runtime は実装済みだが、README が明記している通り現在の smoke lane は guest network path 上で boopa 起点 DHCP を証明していない。`-netdev user` とローカル firmware carrier に依存したままでは DHCP runtime の完成条件が弱いので、guest が boopa の DHCP/TFTP/HTTP を 1 本で踏む検証 lane を最優先で追加する。

## What Changes

- mac ホスト上で直接動く `boopa` をそのまま DHCP 提供側にしつつ、通常ユーザー権限で guest-path E2E を通せる harness を追加する。
- DHCP runtime の acceptance boundary を packet-level tests のみから引き上げ、guest が `DHCP -> TFTP -> grub -> kernel/initrd -> HTTP iso-url` を boopa 経由で通過した証拠を必須にする。
- smoke scripts と backend 起動設定を更新し、guest-visible IP、lease pool、boot endpoints を 1 つの harness から揃えて与える。
- `boopa` を container/VM に移さず、必要なら user 権限で動く `podman` 補助を使える形にする。
- README と operator guidance を更新し、必要な host prerequisites と非対応 fallback を明示する。

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `dhcp-server-runtime`: authoritative DHCP runtime の完了判定を guest-path E2E harness 付きに変更し、boopa 起点の PXE boot chain 証明を要求する。

## Impact

- `scripts/smoke/lib.sh` と `scripts/smoke/test-harness.sh` の network/bootstrap orchestration
- `scripts/smoke/boot-ubuntu-uefi.sh` の authoritative DHCP smoke entrypoint
- `crates/boopa` の guest-visible boot endpoint 設定、logging、必要なら harness 向け config seams
- `README.md` の DHCP verification boundary と host prerequisite 記述
