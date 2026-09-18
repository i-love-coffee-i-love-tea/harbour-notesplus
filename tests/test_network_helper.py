"""Tests for network interface classification and preferred IP address resolution logic."""
import pytest
from PyQt6.QtNetwork import QHostAddress


def is_public_ipv4(addr_str: str) -> bool:
    addr = QHostAddress(addr_str)
    if addr.isNull():
        return False
    res = addr.toIPv4Address()
    ip = res[0] if isinstance(res, tuple) else res
    b0 = (ip >> 24) & 0xFF
    b1 = (ip >> 16) & 0xFF
    b2 = (ip >> 8) & 0xFF

    if b0 == 0:
        return False
    if b0 == 127:
        return False
    if b0 == 10:
        return False
    if b0 == 172 and (16 <= b1 <= 31):
        return False
    if b0 == 192 and b1 == 168:
        return False
    if b0 == 169 and b1 == 254:
        return False
    if b0 == 100 and (64 <= b1 <= 127):
        return False
    if b0 == 192 and b1 == 0 and b2 == 0:
        return False
    if b0 == 192 and b1 == 0 and b2 == 2:
        return False
    if b0 == 198 and (b1 in (18, 19)):
        return False
    if b0 == 198 and b1 == 51 and b2 == 100:
        return False
    if b0 == 203 and b1 == 0 and b2 == 113:
        return False
    if b0 >= 224:
        return False
    if ip == 0xFFFFFFFF:
        return False
    return True


def is_wifi_interface(name: str) -> bool:
    lower = name.lower()
    return lower.startswith("wlan") or lower.startswith("wl") or lower.startswith("wifi")


def is_bluetooth_interface(name: str) -> bool:
    lower = name.lower()
    return lower.startswith("bnep") or lower.startswith("bt")


def is_wan_interface(name: str) -> bool:
    lower = name.lower()
    return (
        lower.startswith("ccmni")
        or lower.startswith("rmnet")
        or lower.startswith("wwan")
        or lower.startswith("cellular")
        or lower.startswith("ppp")
        or lower.startswith("wan")
    )


def resolve_preferred_host_from_candidates(
    reject_public_networks: bool,
    wan_public_ip: str,
    wifi_ip: str,
    bt_ip: str,
) -> str:
    # Priority 1: public if IP WAN enabled and IP available and public IPs not blocked
    if not reject_public_networks and wan_public_ip:
        return wan_public_ip

    # Priority 2: WiFi IP, if available
    if wifi_ip:
        return wifi_ip

    # Priority 3: Bluetooth IP, if available
    if bt_ip:
        return bt_ip

    # Priority 4: localhost
    return "localhost"


# ── Public IPv4 checking ──────────────────────────────────────

def test_is_public_ipv4():
    # Private / Local / Reserved ranges
    assert not is_public_ipv4("127.0.0.1")
    assert not is_public_ipv4("10.0.0.1")
    assert not is_public_ipv4("172.16.0.1")
    assert not is_public_ipv4("172.31.255.255")
    assert not is_public_ipv4("192.168.1.1")
    assert not is_public_ipv4("169.254.1.1")
    assert not is_public_ipv4("100.64.0.1")
    assert not is_public_ipv4("100.127.255.255")
    assert not is_public_ipv4("0.0.0.0")
    assert not is_public_ipv4("224.0.0.1")
    assert not is_public_ipv4("240.0.0.1")
    assert not is_public_ipv4("255.255.255.255")

    # Real public routable addresses
    assert is_public_ipv4("8.8.8.8")
    assert is_public_ipv4("93.184.216.34")
    assert is_public_ipv4("1.1.1.1")
    assert is_public_ipv4("82.165.45.12")


# ── Interface classifiers ────────────────────────────────────

def test_interface_classifiers():
    assert is_wifi_interface("wlan0")
    assert is_wifi_interface("wlp2s0")
    assert is_wifi_interface("wifi0")
    assert not is_wifi_interface("eth0")
    assert not is_wifi_interface("bnep0")

    assert is_bluetooth_interface("bnep0")
    assert is_bluetooth_interface("bt0")
    assert not is_bluetooth_interface("wlan0")

    assert is_wan_interface("ccmni0")
    assert is_wan_interface("rmnet_data0")
    assert is_wan_interface("wwan0")
    assert is_wan_interface("cellular0")
    assert is_wan_interface("ppp0")
    assert not is_wan_interface("wlan0")
    assert not is_wan_interface("bnep0")


# ── Preferred Host Priority Resolution ───────────────────────

def test_priority1_public_ip_wan_allowed():
    # Priority 1: public if IP WAN enabled and IP available and public IPs not blocked
    result = resolve_preferred_host_from_candidates(
        reject_public_networks=False,
        wan_public_ip="82.165.45.12",
        wifi_ip="192.168.1.100",
        bt_ip="192.168.2.15",
    )
    assert result == "82.165.45.12"


def test_priority1_blocked_falls_to_wifi():
    # When block public IPs is enabled, falls to WiFi
    result = resolve_preferred_host_from_candidates(
        reject_public_networks=True,
        wan_public_ip="82.165.45.12",
        wifi_ip="192.168.1.100",
        bt_ip="192.168.2.15",
    )
    assert result == "192.168.1.100"


def test_priority2_wifi_no_wan_public():
    # No public WAN IP (e.g. WAN has private CGNAT IP), WiFi is available
    result = resolve_preferred_host_from_candidates(
        reject_public_networks=False,
        wan_public_ip="",
        wifi_ip="192.168.1.100",
        bt_ip="192.168.2.15",
    )
    assert result == "192.168.1.100"


def test_priority3_bt_when_wifi_unavailable():
    # WiFi not available, Bluetooth available
    result = resolve_preferred_host_from_candidates(
        reject_public_networks=True,
        wan_public_ip="82.165.45.12",
        wifi_ip="",
        bt_ip="192.168.2.15",
    )
    assert result == "192.168.2.15"


def test_priority4_localhost_when_none_available():
    # Neither public WAN, WiFi, nor BT IP available
    result = resolve_preferred_host_from_candidates(
        reject_public_networks=False,
        wan_public_ip="",
        wifi_ip="",
        bt_ip="",
    )
    assert result == "localhost"

    # Block public networks enabled and no WiFi and no BT
    result_blocked = resolve_preferred_host_from_candidates(
        reject_public_networks=True,
        wan_public_ip="82.165.45.12",
        wifi_ip="",
        bt_ip="",
    )
    assert result_blocked == "localhost"
