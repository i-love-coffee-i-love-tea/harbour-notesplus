# ADR-006: Socket-Level Public IP Rejection and Session Persistence Strategy

#### Status
Accepted

#### Context
Sailfish OS devices frequently switch between private home Wi-Fi networks, mobile cellular data connections (4G/5G), and public Wi-Fi access points. Running an embedded HTTP server on a mobile phone without strict network gating exposes the application to port scans, unauthorized remote access, and potential denial-of-service battery drain from public Internet networks.

Furthermore, requiring users to re-enter basic authentication credentials every time the mobile app restarts creates user friction.

#### Decision
Implement a multi-layered security and session persistence strategy:

1. **Default-On Public IP Rejection**: The embedded web server inspects peer IP addresses immediately upon TCP connection establishment:
   - Evaluates peer addresses against RFC 1918 (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16), loopback (127.0.0.0/8, ::1), link-local (169.254.0.0/16, fe80::/10), CGNAT (100.64.0.0/10), IPv6 ULA (fc00::/7), and IPv4-mapped IPv6 ranges.
   - Non-private/public connections are rejected immediately using `SO_LINGER(0)` + TCP `RST` before allocating HTTP buffers, parsing TLS headers, or processing requests.
2. **Persistent Session Store**: Active authenticated web sessions are written atomically to disk (`sessions.json`) using secure random tokens (SHA-256 / CSPRNG) with expiration timestamps, surviving application restarts.

#### Consequences
##### Positive / Utility Delivered
- **Zero-Cost Defense Against Remote Scans**: Public network connections are dropped at the kernel/socket layer, preventing resource exhaustion and DDoS attacks over cellular networks.
- **Frictionless Local Workflow**: Valid web companion sessions persist across app restarts without compromising security.
- **Configurable Control**: Users can adjust the "Reject connections from public networks" toggle in Settings if custom reverse proxying or VPN tunneling is required.

##### Trade-offs / Mitigations
- In environments behind complex corporate NATs or non-standard subnets, users can disable public network rejection in Settings if legitimate connections are classified as public.
