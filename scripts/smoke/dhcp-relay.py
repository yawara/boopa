#!/usr/bin/env python3

import argparse
import selectors
import socket
import time


def parse_args():
    parser = argparse.ArgumentParser(description="Relay DHCP packets from a privileged podman port to boopa on the mac host.")
    parser.add_argument("--listen-host", default="0.0.0.0")
    parser.add_argument("--listen-port", type=int, default=67)
    parser.add_argument("--upstream-host", required=True)
    parser.add_argument("--upstream-port", type=int, required=True)
    parser.add_argument("--client-port", type=int, default=68)
    parser.add_argument("--mapping-ttl-secs", type=int, default=120)
    return parser.parse_args()


def transaction_id(packet: bytes) -> bytes | None:
    if len(packet) < 8:
        return None
    return packet[4:8]


def broadcast_requested(packet: bytes) -> bool:
    if len(packet) < 12:
        return False
    return int.from_bytes(packet[10:12], "big") & 0x8000 != 0


def prune_mappings(mappings: dict[bytes, tuple[str, int, float, bool]], ttl: int) -> None:
    now = time.time()
    stale = [key for key, (_, _, seen_at, _) in mappings.items() if now - seen_at > ttl]
    for key in stale:
        mappings.pop(key, None)


def main() -> None:
    args = parse_args()

    downstream = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    downstream.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    downstream.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
    downstream.bind((args.listen_host, args.listen_port))

    upstream = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    upstream.bind(("0.0.0.0", 0))

    selector = selectors.DefaultSelector()
    selector.register(downstream, selectors.EVENT_READ, "downstream")
    selector.register(upstream, selectors.EVENT_READ, "upstream")

    mappings: dict[bytes, tuple[str, int, float, bool]] = {}

    while True:
        for key, _ in selector.select(timeout=1.0):
            role = key.data
            if role == "downstream":
                packet, client_addr = downstream.recvfrom(4096)
                xid = transaction_id(packet)
                if xid is None:
                    continue
                mappings[xid] = (
                    client_addr[0],
                    args.client_port,
                    time.time(),
                    broadcast_requested(packet),
                )
                upstream.sendto(packet, (args.upstream_host, args.upstream_port))
            else:
                packet, _ = upstream.recvfrom(4096)
                xid = transaction_id(packet)
                if xid is None or xid not in mappings:
                    continue
                host, port, _, should_broadcast = mappings[xid]
                destination = ("255.255.255.255", port) if should_broadcast else (host, port)
                downstream.sendto(packet, destination)

        prune_mappings(mappings, args.mapping_ttl_secs)


if __name__ == "__main__":
    main()
