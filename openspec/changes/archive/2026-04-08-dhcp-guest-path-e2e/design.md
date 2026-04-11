## Context

現在の Ubuntu UEFI smoke harness は `scripts/smoke/lib.sh` で最小 FAT boot volume を用意し、QEMU を `-netdev user` で起動している。そのため guest は boopa の authoritative DHCP runtime から lease や PXE boot options を受け取らず、README でも「current smoke は boopa-origin DHCP inside the guest network path を証明していない」と明記されている。

一方で boopa 自体は authoritative DHCP runtime、TFTP 配信、generated `grub.cfg`、`kernel` / `initrd` / `iso-url` の HTTP 配信を持っている。足りないのはそれらを同じ guest-visible network segment 上で結び、失敗時にどの段階で切れたか分かる形で証拠を残す harness である。

## Goals / Non-Goals

**Goals:**
- Ubuntu UEFI guest が boopa の authoritative DHCP から lease と PXE boot metadata を受ける実ネットワーク smoke lane を追加する。
- `DHCP -> TFTP -> grub -> kernel/initrd -> HTTP iso-url` の各段階が boopa 起点であることを、自動化された evidence で判定できるようにする。
- packet-level DHCP tests は残しつつ、release acceptance は guest-path E2E を必須にする。
- `boopa` は mac ホスト上で直接動かしたまま検証できるようにする。
- host prerequisite 不足時は early fail し、`-netdev user` 等の DHCPless path に silently fallback しない。
- `sudo` や root 前提を要求せず、必要なら user 権限で動く `podman` 補助までを許容範囲にする。

**Non-Goals:**
- 初版で Ubuntu UEFI 以外の distro / boot mode まで guest-path E2E を広げること
- proxy-DHCP、static reservations、multi-subnet 対応
- 汎用的な host network manager を boopa 本体に持たせること
- 既存の fast packet-level tests を削除すること

## Decisions

### 1. Guest-path smoke lane は mac 直動の `boopa` を前提に isolated network backend を明示的に選び、silent fallback を禁止する

`SMOKE_NETWORK_MODE` のような明示的 backend 選択を追加し、acceptance に使う lane は mac ホスト上で直接動く `boopa` から到達できる guest-usable backend でのみ成功扱いにする。必要な user-space helper、address plan、guest attachment が揃わない場合は QEMU 起動前に fail fast する。

Alternatives considered:
- 既存 `-netdev user` lane をそのまま拡張する: rejected because boopa-origin DHCP を guest path で証明できない。
- backend 選択に失敗したら `user` networking へ自動 fallback する: rejected because acceptance が再び曖昧になる。
- `boopa` を Linux VM / container に移してそこから DHCP を出す: rejected because user requirement is that `boopa` itself remains directly on the mac host.

### 2. Harness が guest-visible address plan を一元管理し、mac 上の `boopa` へ明示注入する

Harness は guest subnet、host-side service IP、DHCP pool、router、TFTP advertise address を一括決定し、mac 上で直接動く boopa をその値で起動する。`BOOPA_TFTP_ADVERTISE_ADDR` を guest-visible host IP の canonical source にし、DHCP `next-server` と generated `iso-url` が同じ host identity を指すようにする。補助側は user 権限で動く helper / `podman` までを許容し、boopa 本体は container/VM に移さない。

Alternatives considered:
- host OS の既存アドレスを自動推測する: rejected because wildcard bind や NAT host alias が guest から到達不能な場合がある。
- DHCP 用と HTTP/TFTP 用で別々の advertise source を持つ: rejected because boot chain proof が複雑化し、設定 drift を招く。
- root 権限の bridge/pf/vmnet セットアップを harness の前提にする: rejected because user explicitly requires ordinary user permissions only.

### 3. Acceptance evidence は packet capture ではなく service-native markers を優先する

新規依存や host privilege を増やさずに保守できるよう、まずは backend logs、lease state、asset access logs、guest serial markers を組み合わせて `offer/ack`, `grub`, `kernel/initrd`, `iso-url` の踏破を証明する。必要なら pcap は補助 artifact として後続追加できる形に留める。

Alternatives considered:
- `tcpdump` / pcap を必須にする: rejected because host dependency と privilege burden が増え、CI portability が下がる。
- guest serial markers だけで判定する: rejected because DHCP/TFTP/HTTP のどこを boopa が本当に担ったかが弱い。

### 4. 初版 guest-path E2E は Ubuntu UEFI のみを authoritative target にし、mac 開発体験を優先する

既存 smoke 実装と generated `grub.cfg` path が既に Ubuntu UEFI に集中しているため、最初の real-network lane も同じ target に絞る。他 distro / BIOS 展開は同じ harness seam を再利用する follow-on とする。

Alternatives considered:
- 既存 smoke entrypoint 全種を同時に E2E 化する: rejected because変更が広がり過ぎ、最優先の DHCP proof が遅れる。

## Risks / Trade-offs

- [Host network prerequisites differ by platform] -> Harness backend を明示選択式にし、README に supported / unsupported shape を書き分けて fail-fast する。
- [Real DHCP may be hard from an unprivileged mac host] -> まずは `boopa` を mac 上で直接動かしたまま user 権限で届く helper path に絞って feasibility を詰め、root-only shape は初版成功条件から外す。
- [boopa logs are not yet phase-specific enough] -> DHCP offer/ack、TFTP asset serve、HTTP boot asset fetch に最小限の tracing を追加し、smoke assertions が読む marker を安定化する。
- [The current FAT firmware carrier may still be needed for first-stage UEFI entry] -> それ自体は許容するが、carrier は firmware handoff のみとし、DHCP/TFTP/HTTP path の代替にならないよう制限する。
- [Guest-path lane is slower and harder to debug than packet tests] -> fast protocol tests は残し、slow lane は README と script output に artifact path を残して triage しやすくする。

## Migration Plan

1. 現行 packet-level DHCP tests と `user`-network smoke は保持しつつ、mac 直動 `boopa` 前提の new guest-path lane を追加する。
2. README の DHCP verification boundary を guest-path E2E 必須へ更新する。
3. CI / operator 手順では guest-path lane を authoritative acceptance とし、prereq がない host では fail-fast して skipped success を出さない。

## Open Questions

- `podman` 補助で mac 上の `boopa` から到達できる guest network をどう切るか。
- boopa 側で追加する証跡を structured log にするか、smoke 専用の status artifact へ出すか。
