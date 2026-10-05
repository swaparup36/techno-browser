# Techno Browser over WSS on port 443

This build uses the open-source RustDesk server at commit
`a7736be5e40f85bfc141120dce587e836e5d4b80` plus the local
[`server.patch`](server.patch). It adds desktop registration and rendezvous
message delivery to hbbs's existing WebSocket listener. The unmodified hbbr
WebSocket listener handles relayed desktop traffic. Caddy terminates TLS and
routes both listeners through TCP 443.

The Techno Browser desktop build enables the `wss-only` Cargo feature by
default. Both the controlling and controlled devices must use WSS for
registration and relay. Clients using the usual OSS UDP settings cannot reach
the loopback-only hbbs in this deployment.

## Deploy on EC2

1. Point a DNS name at the instance and allow inbound TCP 443 in its security
   group. Reserve port 443 for this stack. The ACME TLS-ALPN challenge also
   uses port 443, so this Caddy configuration does not need inbound port 80.
2. Copy this repository and your existing `data/` directory to the instance.
   In the repository root, set `RUSTDESK_DOMAIN` to the DNS name and start the
   override Compose file:

   ```sh
   export RUSTDESK_DOMAIN=desk.example.com
   docker compose -f compose.yml -f compose.wss.yml up -d --build
   ```

3. Wait for Caddy to obtain a public TLS certificate. Check `docker compose
   -f compose.yml -f compose.wss.yml logs caddy hbbs hbbr` if registration
   does not start.
4. In Techno Browser's Connect page, set **ID server** to the DNS name, leave
   **Relay server** empty, and set **Key** to the contents of
   `data/id_ed25519.pub`. The page fills an empty API server with
   `https://<DNS name>` and enables WebSocket. The API server URL is used to
   select WSS; this stack serves the RustDesk WebSocket routes, not the Pro
   account API.

The only public RustDesk transport in this configuration is WSS on TCP 443.
DNS lookups, certificate issuance, updates, and normal web pages use their own
network traffic. WSS starts as an HTTPS upgrade and carries binary WebSocket
frames afterward.

## Verify

Run the included server integration test from a checkout of the pinned OSS
server after applying `server.patch`:

```sh
cargo test --test ws_registration
```

The test registers a device over WebSocket, then confirms a controller's
punch request reaches it through hbbs. It does not require a Pro license.
