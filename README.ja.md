# boopa

## 警告

**初期段階のソフトウェアです。本番環境では使用しないでください。**

このプロジェクトはまだ初期段階です。破壊的変更、未成熟なハードニング、不十分な運用上の安全策、予告なく変わる挙動を前提にしてください。

![boopa](./boopa.png)

`boopa` は、信頼できるオフィス LAN 向けの単一サービス型ネットワークブートコントローラです。HTTP と TFTP でブートアセットを配信し、必要に応じて限定された PXE サブネット向けの authoritative DHCPv4 サーバとして動作し、現在選択されているディストリビューションを永続化し、DHCP 状態とブート手順を一か所で確認できる小さなダッシュボード/API を提供します。

## スコープ

- `actix-web` ベースの HTTP サービスと内蔵 TFTP サービスを持つ Rust バックエンド
- React + TypeScript + RTK Query ダッシュボード
- v1 対応ディストリビューション: Ubuntu, Fedora, Arch Linux
- 対応ブートモード: BIOS と UEFI
- 公式 upstream アセットをキャッシュして配信
- Ubuntu カスタムイメージのビルドを、制限付きの v1 ビルドレーンとしてサポート: Linux ホスト限定、root 必須、backendless な Ubuntu UEFI smoke path で検証
- `boopa` 自体はネットワークブートコントローラのままであり、カスタムイメージビルドは別のビルドレーンであって、ランタイム配信パスではない

現行リリースでスコープ外:

- Proxy-DHCP / DHCP assist mode
- Static reservation / MAC 固定リース
- 認証またはアクセス制御
- インストール後自動化
- Ubuntu 以外のカスタムイメージビルド
- Linux 以外、または rootless な custom-image ビルドサポート

## ランタイム前提

- 信頼できる LAN 上、または localhost 限定の tunnel/proxy の背後でデプロイする
- DHCP の `next-server` と boot filename は手動でこのサービスへ向ける
- フロントエンドは事前に `frontend/dist` へビルドするか、`BOOPA_FRONTEND_DIR` で上書きする

## 環境変数

- `BOOPA_API_BIND` 既定値: `127.0.0.1:8080`
- `BOOPA_TFTP_BIND` 既定値: `0.0.0.0:6969`
- `BOOPA_TFTP_ADVERTISE_ADDR` 既定値: TFTP bind address が guest から到達可能ならその値、そうでなければ `127.0.0.1:<tftp-port>`
- `BOOPA_DHCP_MODE` 既定値: `disabled`
- `BOOPA_DHCP_BIND` 既定値: `0.0.0.0:67`
- `BOOPA_DHCP_SUBNET`: DHCP mode が `authoritative` のとき必須（例: `10.0.2.0/24`）
- `BOOPA_DHCP_POOL_START` / `BOOPA_DHCP_POOL_END`: DHCP mode が `authoritative` のとき必須
- `BOOPA_DHCP_ROUTER`: リースへ配る IPv4 デフォルトゲートウェイ。任意
- `BOOPA_DHCP_DNS`: リースへ配るカンマ区切り IPv4 DNS サーバ。任意
- `BOOPA_DHCP_LEASE_SECS` 既定値: `3600`
- `BOOPA_DATA_DIR` 既定値: `var/boopa`
- `BOOPA_FRONTEND_DIR` 既定値: `frontend/dist`

## API

- `GET /api/health`
- `GET /api/distros`
- `GET /api/dhcp` は、手動 BIOS/UEFI ガイドと現在の DHCP ランタイム状態の両方を返す
- `PUT /api/selection`
- `GET /api/cache`
- `POST /api/cache/refresh`

DHCP mode に関する補足:

- DHCP は既定で無効
- `BOOPA_DHCP_MODE=authoritative` の場合、boopa は 1 つの IPv4 サブネットに対して動的リースのみを提供する
- Proxy-DHCP と static reservation は意図的に後回しにしている

キャッシュ更新の挙動:

- キャッシュ更新は手動のみ。`POST /api/cache/refresh` が呼ばれたときにアセットを更新する
- `boopa` はアセットハッシュを `BOOPA_DATA_DIR/cache/manifest.json` に保存する
- レシピのアセットファイルがすでに存在し、保存済み SHA-256 と `source_url` が一致していれば、更新時にそのアセットの再ダウンロードをスキップする
- ファイルが存在しない、ハッシュが違う、またはレシピの `source_url` が変わっている場合は、再ダウンロードして manifest を更新する
- `force` refresh は未実装

## 検証

バックエンド:

- `cargo fmt --check`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo build --workspace`

フロントエンド:

- `npm ci --prefix frontend`
- `npm run dev --prefix frontend`
- `npm test --prefix frontend`
- `npm run typecheck --prefix frontend`
- `npm run build --prefix frontend`
- `npx --prefix frontend playwright install chromium`
- `npm run test:e2e --prefix frontend`

現行リリースにおける DHCP 検証:

- authoritative DHCP runtime に対する高速な回帰下限は、引き続き packet-level DHCP tests
- 動作確認済みの guest-path acceptance lane は、mac-host 上の `SMOKE_NETWORK_MODE=vde` smoke path。ここでは `boopa` 自体は macOS 上で動いたままで、user-space の VDE helper が guest の DHCP/TFTP/HTTP トラフィックをホスト上のプロセスへ橋渡しする
- `SMOKE_NETWORK_MODE=vmnet-host` は引き続き実験的な fallback。だが、この host/QEMU の組み合わせでは追加の権限や entitlement がないと vmnet interface 作成に失敗する
- 旧来の `-netdev user` smoke path を、guest network path 内での boopa 起点 DHCP の証明として扱ってはいけない

フロントエンド dev proxy:

- フロントエンドの npm package と lockfile はどちらも `frontend/` 配下にある
- `npm run dev --prefix frontend` は `/api` と `/boot` を `http://127.0.0.1:8080` へ proxy する
- 開発用バックエンドの接続先は `BOOPA_DEV_BACKEND=http://host:port npm run dev --prefix frontend` で上書きできる

フロントエンド e2e:

- `frontend/playwright.config.ts` は、ブラウザテスト用に実際の `boopa` バックエンドと Vite dev server の両方を起動する
- ブラウザレーンは OS の一時ディレクトリ配下に分離した `BOOPA_DATA_DIR` を使い、既定の `var/boopa` は再利用しない
- 初期段階のブラウザカバレッジは意図的に絞っている:
  - live backend に対するダッシュボード初期表示
  - Ubuntu autoinstall の編集/保存
  - 保存された autoinstall state の reload 後永続化
- 初期段階のブラウザカバレッジには distro-switch e2e は含めない
- 初期段階のブラウザカバレッジでは、backend が配る静的アセットのブラウザ内検証は行わない。Vite dev server と live backend path を対象にしている

典型的なローカル frontend e2e 検証:

```sh
npx --prefix frontend playwright install chromium
npm run test:e2e --prefix frontend
```

Smoke CLI:

- `python3 -m scripts.smoke run --distro ubuntu --boot-mode uefi`
- `python3 -m scripts.smoke run --distro ubuntu --boot-mode bios`
- `python3 -m scripts.smoke run --distro fedora --boot-mode uefi`
- `python3 -m scripts.smoke run --distro fedora --boot-mode bios`
- `python3 -m scripts.smoke custom-image`
- `python3 -m scripts.smoke.test_harness`

## 補足

smoke scripts は、QEMU ベースの検証レーンに対する構造化 entrypoint です。ローカル hypervisor ツールと upstream boot asset へのネットワーク到達性が必要なので、通常のローカル build の前提ではなく、operator/CI 用スクリプトとして扱ってください。

具体的な harness の現在の範囲:

- canonical surface は `python3 -m scripts.smoke`。legacy shell entrypoint は削除済み
- 正式な target coverage は `Ubuntu/Fedora x UEFI/BIOS`。`custom-image` は Ubuntu UEFI 専用レーンとして維持
- `Arch` は supported matrix に含まれない
- harness は dry-run 時に構造化された execution plan (`logs/plan.json`) を出力するので、reviewer は shell 内部を読まずに commands, helper processes, side effects を確認できる
- guest-path backend は `SMOKE_NETWORK_MODE` で選ぶ
  - `user` は従来の debug/support path であり、DHCP acceptance ではない
  - `vde` は現在の mac-host acceptance path。user-space の `vde_switch` と host helper を起動し、`boopa` はホスト上で動き続ける
  - `vmnet-host` は引き続き `SMOKE_DHCP_HELPER_MODE=podman-relay` と組み合わせる実験的 backend。だが host/QEMU の組み合わせによっては追加の権限や entitlement がないと vmnet interface 作成に失敗する
- BIOS target は support matrix と planner 上では明示的にモデル化されているが、このホストで representative な BIOS smoke lane を end-to-end 実行するまでは live execution は未検証
- smoke 実行中、`BOOPA_DATA_DIR/cache` は `var/boopa/cache`（または `SMOKE_SOURCE_DATA_DIR/cache`）への symlink になるので、キャッシュ済みアセットと `manifest.json` を再利用する
- 初段の firmware handoff 用に一時 FAT boot volume が必要な場合、その中身は firmware-carrier files と、`boopa` が配る bootloader / GRUB config のコピーに限定される
- `boopa` は Ubuntu UEFI の `grub.cfg` を生成して配信する。kernel と initrd は TFTP 経由で `ubuntu/uefi/kernel` と `ubuntu/uefi/initrd` として取得し、生成された `iso-url` は `/boot/ubuntu/uefi/live-server.iso` を HTTP 経由で指す
- Ubuntu UEFI client は、広告された TFTP endpoint と `http://<boopa-host>:<api-port>/boot/ubuntu/uefi/live-server.iso` の両方へ到達できる必要がある
- mac-host guest-path lane では、`boopa` は DHCP を unprivileged localhost port に bind し、選択された helper backend が guest traffic をそのホストプロセスへ橋渡しする。これにより `boopa` を VM や container へ移さず、通常ユーザー権限のまま完結できる
- Ubuntu UEFI smoke path は既定で `RAM_MB=8192` を使い、`SYSTEM_DISK_GB=32` の qcow2 installer disk を用意する。これは live installer が live filesystem へ pivot する前に数 GB の ISO をダウンロードするため
- smoke harness は既定でランダムな高位の unprivileged API/TFTP port を選び、ローカルの port 衝突を避ける
- interactive terminal から起動した場合、harness は QEMU の serial I/O をその terminal へ接続し、既定で QEMU display window も有効にする。VGA/installer 出力を見たいときに使う。headless を強制するには `SMOKE_INTERACTIVE=0`、display backend を変えるには `SMOKE_QEMU_DISPLAY` を使う
- 成功判定はログベース。ideal marker は installer/live の進行を示し、fallback marker は kernel/initrd handoff と boot 継続を示す

canonical custom-image smoke の形:

- `CUSTOM_IMAGE_BASE_ISO`, `CUSTOM_IMAGE_MANIFEST`, `CUSTOM_IMAGE_OUTPUT_ISO` を設定する
- `python3 -m scripts.smoke custom-image` を実行する
- そのレーンは必要なら ISO をビルドし、その後 `boopa` を起動せずに生成済み Ubuntu UEFI image をブートする

典型的なローカル smoke 検証:

```sh
python3 -m scripts.smoke run --distro ubuntu --boot-mode uefi
```

典型的な mac-host guest-path smoke の形:

```sh
python3 -m scripts.smoke plan --distro ubuntu --boot-mode uefi --network-mode vde --format json
python3 -m scripts.smoke run --distro ubuntu --boot-mode uefi --network-mode vde
```

harness 自体の dry-run/regression 検証:

```sh
python3 -m scripts.smoke.test_harness
```
