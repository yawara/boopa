#!/usr/bin/env python3

import argparse
import os
import random
import selectors
import socket
import struct
import subprocess
from typing import Dict, Tuple


ETH_P_IP = 0x0800
ETH_P_ARP = 0x0806
ARP_REQUEST = 1
ARP_REPLY = 2
IPPROTO_ICMP = 1
IPPROTO_TCP = 6
IPPROTO_UDP = 17

TCP_FIN = 0x01
TCP_SYN = 0x02
TCP_RST = 0x04
TCP_PSH = 0x08
TCP_ACK = 0x10


def mac_to_bytes(value: str) -> bytes:
    return bytes(int(part, 16) for part in value.split(":"))


def ip_to_bytes(value: str) -> bytes:
    return socket.inet_aton(value)


def ones_complement_sum(data: bytes) -> int:
    if len(data) % 2:
        data += b"\x00"
    total = 0
    for index in range(0, len(data), 2):
        total += (data[index] << 8) + data[index + 1]
        total = (total & 0xFFFF) + (total >> 16)
    return total


def checksum(data: bytes) -> int:
    return (~ones_complement_sum(data)) & 0xFFFF


def build_ethernet(dst_mac: bytes, src_mac: bytes, ether_type: int, payload: bytes) -> bytes:
    return dst_mac + src_mac + struct.pack("!H", ether_type) + payload


def build_arp_reply(src_mac: bytes, src_ip: bytes, dst_mac: bytes, dst_ip: bytes) -> bytes:
    payload = struct.pack(
        "!HHBBH6s4s6s4s",
        1,
        ETH_P_IP,
        6,
        4,
        ARP_REPLY,
        src_mac,
        src_ip,
        dst_mac,
        dst_ip,
    )
    return build_ethernet(dst_mac, src_mac, ETH_P_ARP, payload)


def build_ipv4(src_ip: bytes, dst_ip: bytes, proto: int, payload: bytes, identification: int) -> bytes:
    version_ihl = 0x45
    total_length = 20 + len(payload)
    header = struct.pack(
        "!BBHHHBBH4s4s",
        version_ihl,
        0,
        total_length,
        identification & 0xFFFF,
        0,
        64,
        proto,
        0,
        src_ip,
        dst_ip,
    )
    return header[:10] + struct.pack("!H", checksum(header)) + header[12:] + payload


def build_udp(src_port: int, dst_port: int, src_ip: bytes, dst_ip: bytes, payload: bytes) -> bytes:
    length = 8 + len(payload)
    header = struct.pack("!HHHH", src_port, dst_port, length, 0)
    pseudo = src_ip + dst_ip + struct.pack("!BBH", 0, IPPROTO_UDP, length)
    csum = checksum(pseudo + header + payload)
    if csum == 0:
        csum = 0xFFFF
    return struct.pack("!HHHH", src_port, dst_port, length, csum) + payload


def build_tcp(
    src_port: int,
    dst_port: int,
    seq: int,
    ack: int,
    flags: int,
    src_ip: bytes,
    dst_ip: bytes,
    payload: bytes = b"",
    window: int = 65535,
) -> bytes:
    data_offset = 5
    offset_flags = (data_offset << 12) | flags
    header = struct.pack("!HHIIHHHH", src_port, dst_port, seq, ack, offset_flags, window, 0, 0)
    pseudo = src_ip + dst_ip + struct.pack("!BBH", 0, IPPROTO_TCP, len(header) + len(payload))
    csum = checksum(pseudo + header + payload)
    return struct.pack("!HHIIHHHH", src_port, dst_port, seq, ack, offset_flags, window, csum, 0) + payload


def parse_ethernet(frame: bytes):
    if len(frame) < 14:
        return None
    dst = frame[0:6]
    src = frame[6:12]
    ether_type = struct.unpack("!H", frame[12:14])[0]
    return dst, src, ether_type, frame[14:]


def parse_arp(payload: bytes):
    if len(payload) < 28:
        return None
    return struct.unpack("!HHBBH6s4s6s4s", payload[:28])


def parse_ipv4(payload: bytes):
    if len(payload) < 20:
        return None
    version_ihl = payload[0]
    ihl = (version_ihl & 0x0F) * 4
    if len(payload) < ihl:
        return None
    total_length = struct.unpack("!H", payload[2:4])[0]
    proto = payload[9]
    src_ip = payload[12:16]
    dst_ip = payload[16:20]
    return {
        "ihl": ihl,
        "proto": proto,
        "src_ip": src_ip,
        "dst_ip": dst_ip,
        "payload": payload[ihl:total_length],
    }


def parse_udp(payload: bytes):
    if len(payload) < 8:
        return None
    src_port, dst_port, length, _checksum = struct.unpack("!HHHH", payload[:8])
    return {
        "src_port": src_port,
        "dst_port": dst_port,
        "payload": payload[8:length],
    }


def parse_tcp(payload: bytes):
    if len(payload) < 20:
        return None
    src_port, dst_port, seq, ack, offset_flags, window, _checksum, _urg = struct.unpack(
        "!HHIIHHHH", payload[:20]
    )
    data_offset = ((offset_flags >> 12) & 0xF) * 4
    flags = offset_flags & 0x1FF
    if len(payload) < data_offset:
        return None
    return {
        "src_port": src_port,
        "dst_port": dst_port,
        "seq": seq,
        "ack": ack,
        "flags": flags,
        "window": window,
        "payload": payload[data_offset:],
    }


def build_ipv4_udp_frame(
    src_mac: bytes,
    dst_mac: bytes,
    src_ip: bytes,
    dst_ip: bytes,
    src_port: int,
    dst_port: int,
    payload: bytes,
    identification: int,
) -> bytes:
    udp = build_udp(src_port, dst_port, src_ip, dst_ip, payload)
    ip = build_ipv4(src_ip, dst_ip, IPPROTO_UDP, udp, identification)
    return build_ethernet(dst_mac, src_mac, ETH_P_IP, ip)


def build_ipv4_tcp_frame(
    src_mac: bytes,
    dst_mac: bytes,
    src_ip: bytes,
    dst_ip: bytes,
    src_port: int,
    dst_port: int,
    seq: int,
    ack: int,
    flags: int,
    identification: int,
    payload: bytes = b"",
) -> bytes:
    tcp = build_tcp(src_port, dst_port, seq, ack, flags, src_ip, dst_ip, payload)
    ip = build_ipv4(src_ip, dst_ip, IPPROTO_TCP, tcp, identification)
    return build_ethernet(dst_mac, src_mac, ETH_P_IP, ip)


class UdpFlow:
    def __init__(self, guest_mac: bytes, guest_ip: bytes, guest_port: int, remote_port: int, upstream_sock: socket.socket):
        self.guest_mac = guest_mac
        self.guest_ip = guest_ip
        self.guest_port = guest_port
        self.remote_port = remote_port
        self.upstream_sock = upstream_sock
        self.logged_tftp_blocks = set()
        self.logged_udp_events = 0


class TcpFlow:
    def __init__(self, guest_mac: bytes, guest_ip: bytes, guest_port: int, host_port: int, upstream_sock: socket.socket):
        self.guest_mac = guest_mac
        self.guest_ip = guest_ip
        self.guest_port = guest_port
        self.host_port = host_port
        self.upstream_sock = upstream_sock
        self.guest_next_seq = 0
        self.host_seq = random.randrange(0, 2**32)
        self.sent_fin = False


class Helper:
    def __init__(self, args):
        self.args = args
        self.selector = selectors.DefaultSelector()
        self.host_mac = mac_to_bytes(args.host_mac)
        self.host_ip = ip_to_bytes(args.host_ip)
        self.broadcast_mac = b"\xff" * 6
        self.broadcast_ip = ip_to_bytes("255.255.255.255")
        self.ip_id = random.randrange(0, 65535)
        self.vde = subprocess.Popen(
            ["vde_plug", args.switch_dir],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            bufsize=0,
        )
        self.read_buffer = bytearray()
        self.selector.register(self.vde.stdout, selectors.EVENT_READ, ("vde", None))

        self.dhcp_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.dhcp_sock.bind(("127.0.0.1", 0))
        self.dhcp_sock.setblocking(False)
        self.selector.register(self.dhcp_sock, selectors.EVENT_READ, ("dhcp", None))

        self.dhcp_transactions: Dict[bytes, Tuple[bytes, bytes]] = {}
        self.udp_guest_flows: Dict[int, UdpFlow] = {}
        self.udp_upstream_flows: Dict[socket.socket, UdpFlow] = {}
        self.tcp_guest_flows: Dict[int, TcpFlow] = {}
        self.tcp_upstream_flows: Dict[socket.socket, TcpFlow] = {}

    def log(self, message: str) -> None:
        print(message, flush=True)

    def next_ip_id(self) -> int:
        self.ip_id = (self.ip_id + 1) & 0xFFFF
        return self.ip_id

    def send_frame(self, frame: bytes) -> None:
        payload = struct.pack("!H", len(frame)) + frame
        self.vde.stdin.write(payload)
        self.vde.stdin.flush()

    def run(self) -> None:
        try:
            while True:
                for key, _mask in self.selector.select(timeout=1.0):
                    role, _obj = key.data
                    if role == "vde":
                        self.handle_vde_read()
                    elif role == "dhcp":
                        self.handle_dhcp_response()
                    elif role == "udp":
                        self.handle_udp_response(key.fileobj)
                    elif role == "tcp":
                        self.handle_tcp_response(key.fileobj)
        finally:
            try:
                self.vde.terminate()
            except Exception:
                pass

    def handle_vde_read(self) -> None:
        chunk = os.read(self.vde.stdout.fileno(), 8192)
        if not chunk:
            raise SystemExit(0)
        self.read_buffer.extend(chunk)

        while len(self.read_buffer) >= 2:
            frame_len = struct.unpack("!H", self.read_buffer[:2])[0]
            if len(self.read_buffer) < 2 + frame_len:
                return
            frame = bytes(self.read_buffer[2 : 2 + frame_len])
            del self.read_buffer[: 2 + frame_len]
            self.handle_frame(frame)

    def handle_frame(self, frame: bytes) -> None:
        parsed_eth = parse_ethernet(frame)
        if parsed_eth is None:
            return
        _dst_mac, src_mac, ether_type, payload = parsed_eth

        if ether_type == ETH_P_ARP:
            arp = parse_arp(payload)
            if arp is None:
                return
            _htype, _ptype, _hlen, _plen, oper, sha, spa, _tha, tpa = arp
            if oper == ARP_REQUEST and tpa == self.host_ip:
                self.send_frame(build_arp_reply(self.host_mac, self.host_ip, sha, spa))
            return

        if ether_type != ETH_P_IP:
            return

        ipv4 = parse_ipv4(payload)
        if ipv4 is None:
            return
        if ipv4["dst_ip"] != self.host_ip and ipv4["dst_ip"] != self.broadcast_ip:
            return

        if ipv4["proto"] == IPPROTO_UDP:
            udp = parse_udp(ipv4["payload"])
            if udp is None:
                return
            self.handle_udp_from_guest(src_mac, ipv4["src_ip"], ipv4["dst_ip"], udp)
        elif ipv4["proto"] == IPPROTO_TCP:
            tcp = parse_tcp(ipv4["payload"])
            if tcp is None:
                return
            self.handle_tcp_from_guest(src_mac, ipv4["src_ip"], tcp)

    def handle_udp_from_guest(self, guest_mac: bytes, guest_ip: bytes, dst_ip: bytes, udp) -> None:
        if udp["dst_port"] == 67:
            xid = udp["payload"][4:8] if len(udp["payload"]) >= 8 else None
            if xid is None:
                return
            self.dhcp_transactions[xid] = (guest_mac, guest_ip)
            self.log(f"dhcp relay xid={xid.hex()} guest_mac={guest_mac.hex(':')}")
            self.dhcp_sock.sendto(
                udp["payload"],
                (self.args.dhcp_upstream_host, self.args.dhcp_upstream_port),
            )
            return

        flow = self.udp_guest_flows.get(udp["src_port"])
        if flow is None:
            upstream_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            upstream_sock.bind(("127.0.0.1", 0))
            upstream_sock.setblocking(False)
            self.selector.register(upstream_sock, selectors.EVENT_READ, ("udp", None))
            flow = UdpFlow(guest_mac, guest_ip, udp["src_port"], udp["dst_port"], upstream_sock)
            self.udp_guest_flows[udp["src_port"]] = flow
            self.udp_upstream_flows[upstream_sock] = flow
        else:
            flow.guest_mac = guest_mac
            flow.guest_ip = guest_ip

        if udp["dst_port"] == self.args.tftp_upstream_port and len(udp["payload"]) > 2:
            opcode = struct.unpack("!H", udp["payload"][:2])[0]
            if opcode == 1:
                parts = udp["payload"][2:].split(b"\x00", 2)
                if parts and parts[0]:
                    path = parts[0].decode("utf-8", errors="replace")
                    self.log(f"tftp rrq path={path}")
        elif len(udp["payload"]) >= 4:
            opcode = struct.unpack("!H", udp["payload"][:2])[0]
            if opcode == 4:
                block = struct.unpack("!H", udp["payload"][2:4])[0]
                if len(flow.logged_tftp_blocks) < 12:
                    self.log(f"tftp ack block={block} remote_port={flow.remote_port}")
        flow.upstream_sock.sendto(
            udp["payload"],
            (self.args.tftp_upstream_host, flow.remote_port),
        )

    def handle_dhcp_response(self) -> None:
        payload, _addr = self.dhcp_sock.recvfrom(4096)
        if len(payload) < 8:
            return
        xid = payload[4:8]
        mapping = self.dhcp_transactions.get(xid)
        if mapping is None:
            return
        guest_mac, _guest_ip = mapping
        self.log(f"dhcp response xid={xid.hex()}")
        frame = build_ipv4_udp_frame(
            self.host_mac,
            self.broadcast_mac,
            self.host_ip,
            self.broadcast_ip,
            67,
            68,
            payload,
            self.next_ip_id(),
        )
        self.send_frame(frame)

    def handle_udp_response(self, upstream_sock: socket.socket) -> None:
        payload, upstream = upstream_sock.recvfrom(65535)
        flow = self.udp_upstream_flows.get(upstream_sock)
        if flow is None:
            return
        flow.remote_port = upstream[1]
        if upstream[1] != 67:
            if len(payload) >= 4 and struct.unpack("!H", payload[:2])[0] == 3:
                block = struct.unpack("!H", payload[2:4])[0]
                if block not in flow.logged_tftp_blocks and len(flow.logged_tftp_blocks) < 12:
                    flow.logged_tftp_blocks.add(block)
                    self.log(f"tftp data block={block} src_port={upstream[1]} guest_port={flow.guest_port}")
            elif flow.logged_udp_events < 4:
                flow.logged_udp_events += 1
                self.log(f"udp upstream src_port={upstream[1]} guest_port={flow.guest_port}")
        frame = build_ipv4_udp_frame(
            self.host_mac,
            flow.guest_mac,
            self.host_ip,
            flow.guest_ip,
            upstream[1],
            flow.guest_port,
            payload,
            self.next_ip_id(),
        )
        self.send_frame(frame)

    def handle_tcp_from_guest(self, guest_mac: bytes, guest_ip: bytes, tcp) -> None:
        flow = self.tcp_guest_flows.get(tcp["src_port"])

        if flow is None:
            if not (tcp["flags"] & TCP_SYN):
                return
            try:
                upstream_sock = socket.create_connection(
                    (self.args.http_upstream_host, tcp["dst_port"]),
                    timeout=2,
                )
            except OSError:
                rst = build_ipv4_tcp_frame(
                    self.host_mac,
                    guest_mac,
                    self.host_ip,
                    guest_ip,
                    tcp["dst_port"],
                    tcp["src_port"],
                    0,
                    tcp["seq"] + 1,
                    TCP_RST | TCP_ACK,
                    self.next_ip_id(),
                )
                self.send_frame(rst)
                return

            upstream_sock.setblocking(False)
            flow = TcpFlow(guest_mac, guest_ip, tcp["src_port"], tcp["dst_port"], upstream_sock)
            self.log(f"tcp connect guest_port={tcp['src_port']} host_port={tcp['dst_port']}")
            flow.guest_next_seq = tcp["seq"] + 1
            self.tcp_guest_flows[tcp["src_port"]] = flow
            self.tcp_upstream_flows[upstream_sock] = flow
            self.selector.register(upstream_sock, selectors.EVENT_READ, ("tcp", None))

            synack = build_ipv4_tcp_frame(
                self.host_mac,
                guest_mac,
                self.host_ip,
                guest_ip,
                tcp["dst_port"],
                tcp["src_port"],
                flow.host_seq,
                flow.guest_next_seq,
                TCP_SYN | TCP_ACK,
                self.next_ip_id(),
            )
            flow.host_seq = (flow.host_seq + 1) & 0xFFFFFFFF
            self.send_frame(synack)
            return

        flow.guest_mac = guest_mac
        flow.guest_ip = guest_ip

        payload = tcp["payload"]
        if payload:
            if flow.host_port == self.args.http_upstream_port:
                request_line = payload.split(b"\r\n", 1)[0]
                if request_line.startswith(b"GET "):
                    self.log(f"http request {request_line.decode('utf-8', errors='replace')}")
            if tcp["seq"] == flow.guest_next_seq:
                flow.upstream_sock.sendall(payload)
                flow.guest_next_seq = (flow.guest_next_seq + len(payload)) & 0xFFFFFFFF
            ack = build_ipv4_tcp_frame(
                self.host_mac,
                flow.guest_mac,
                self.host_ip,
                flow.guest_ip,
                flow.host_port,
                flow.guest_port,
                flow.host_seq,
                flow.guest_next_seq,
                TCP_ACK,
                self.next_ip_id(),
            )
            self.send_frame(ack)

        if tcp["flags"] & TCP_FIN:
            flow.guest_next_seq = (flow.guest_next_seq + 1) & 0xFFFFFFFF
            try:
                flow.upstream_sock.shutdown(socket.SHUT_WR)
            except OSError:
                pass
            ack = build_ipv4_tcp_frame(
                self.host_mac,
                flow.guest_mac,
                self.host_ip,
                flow.guest_ip,
                flow.host_port,
                flow.guest_port,
                flow.host_seq,
                flow.guest_next_seq,
                TCP_ACK,
                self.next_ip_id(),
            )
            self.send_frame(ack)

    def handle_tcp_response(self, upstream_sock: socket.socket) -> None:
        flow = self.tcp_upstream_flows.get(upstream_sock)
        if flow is None:
            return

        try:
            payload = upstream_sock.recv(4096)
        except BlockingIOError:
            return

        if payload:
            frame = build_ipv4_tcp_frame(
                self.host_mac,
                flow.guest_mac,
                self.host_ip,
                flow.guest_ip,
                flow.host_port,
                flow.guest_port,
                flow.host_seq,
                flow.guest_next_seq,
                TCP_ACK | TCP_PSH,
                self.next_ip_id(),
                payload,
            )
            flow.host_seq = (flow.host_seq + len(payload)) & 0xFFFFFFFF
            self.send_frame(frame)
            return

        if not flow.sent_fin:
            fin = build_ipv4_tcp_frame(
                self.host_mac,
                flow.guest_mac,
                self.host_ip,
                flow.guest_ip,
                flow.host_port,
                flow.guest_port,
                flow.host_seq,
                flow.guest_next_seq,
                TCP_ACK | TCP_FIN,
                self.next_ip_id(),
            )
            flow.host_seq = (flow.host_seq + 1) & 0xFFFFFFFF
            flow.sent_fin = True
            self.send_frame(fin)


def parse_args():
    parser = argparse.ArgumentParser(description="User-space host helper for QEMU VDE smoke networking.")
    parser.add_argument("--switch-dir", required=True)
    parser.add_argument("--host-ip", required=True)
    parser.add_argument("--host-mac", default="02:50:00:00:00:01")
    parser.add_argument("--dhcp-upstream-host", required=True)
    parser.add_argument("--dhcp-upstream-port", type=int, required=True)
    parser.add_argument("--tftp-upstream-host", required=True)
    parser.add_argument("--tftp-upstream-port", type=int, required=True)
    parser.add_argument("--http-upstream-host", required=True)
    parser.add_argument("--http-upstream-port", type=int, required=True)
    return parser.parse_args()


def main():
    args = parse_args()
    helper = Helper(args)
    helper.run()


if __name__ == "__main__":
    main()
