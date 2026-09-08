# ADR-007: Zero External CDN Dependency and Vendored Vue 3 ESM Assets

#### Status
Accepted

#### Context
Web companion interfaces frequently fetch JavaScript frameworks and stylesheets from public Content Delivery Networks (CDNs, such as jsDelivr, unpkg, or Google Fonts). 

In a local-first mobile note system, CDN dependencies introduce major operational failures:
1. When operating on local Wi-Fi networks without active upstream Internet access (e.g. mobile hotspot or offline router), external CDN scripts fail to load, rendering the web interface broken.
2. Loading external assets leaks user connection metadata and IP addresses to third-party CDN providers.
3. Unversioned or unpinned CDN scripts expose the system to supply-chain tampering and CDN outages.

#### Decision
Vendor all frontend dependencies locally within `notesplusplus-core/assets/web/`:
1. **Vendored Vue 3 ESM**: Embed `vue.esm-browser.prod.js` directly into the binary assets and map `"vue"` in `index.html` import maps to the local `/vue.esm-browser.prod.js` endpoint.
2. **Local Static Assets**: Bundle all CSS stylesheets, vector SVG icons, and application scripts (`app.js`, `style.css`, `icon.png`) into the core crate using `include_str!` and `include_bytes!`.
3. **Template Directives Guarding**: Utilize `v-cloak` CSS rules (`[v-cloak] { display: none !important; }`) to prevent raw, uncompiled Vue template directives from flashing before script initialization.

#### Consequences
##### Positive / Utility Delivered
- **100% Offline Resilience**: The companion web UI functions seamlessly in air-gapped, offline, and local-only environments.
- **Enhanced Privacy & Security**: Zero external network traffic or metadata leakage to third-party CDNs.
- **Zero-Latency Asset Delivery**: Static assets are served immediately from device memory with optimal cache headers.

##### Trade-offs / Mitigations
- Embedding frontend dependencies slightly increases binary size (~300 KB), which is negligible compared to overall application footprint.
