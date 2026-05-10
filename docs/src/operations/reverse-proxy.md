# Reverse Proxy Setup

`http-smtp-rele` binds to a loopback address and relies on a reverse proxy
for TLS termination, access control, and public-facing HTTP.

> **Alternative:** For direct TLS without a proxy, build with
> `--features tls` and configure `[server].tls_cert` / `tls_key`.
> See [Configuration Reference](../guides/configuration.md).

---

## nginx

### Basic TLS termination

```nginx
server {
    listen 443 ssl;
    server_name relay.example.com;

    ssl_certificate     /etc/ssl/relay.example.com.crt;
    ssl_certificate_key /etc/ssl/relay.example.com.key;
    ssl_protocols       TLSv1.2 TLSv1.3;
    ssl_ciphers         HIGH:!aNULL:!MD5;

    location / {
        proxy_pass       http://127.0.0.1:8080;
        proxy_set_header Host              $host;
        proxy_set_header X-Real-IP         $remote_addr;
        proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Restrict internal-only endpoints
    location /readyz {
        allow 10.0.0.0/8;
        allow 127.0.0.1;
        deny  all;
        proxy_pass http://127.0.0.1:8080;
    }

    location /metrics {
        allow 10.0.0.0/8;   # monitoring network only
        deny  all;
        proxy_pass http://127.0.0.1:8080;
    }
}

# Redirect HTTP → HTTPS
server {
    listen 80;
    server_name relay.example.com;
    return 301 https://$host$request_uri;
}
```

### IP-based client restriction

If your callers are known services:

```nginx
location /v1/ {
    allow 203.0.113.0/24;   # production app servers
    allow 198.51.100.5;     # staging
    deny  all;
    proxy_pass http://127.0.0.1:8080;
}
```

### Trusted proxy headers

When nginx passes `X-Forwarded-For`, configure the relay to trust it:

```toml
[security]
trust_proxy_headers  = true
trusted_source_cidrs = ["127.0.0.1/32"]
```

---

## Caddy

```caddy
relay.example.com {
    reverse_proxy 127.0.0.1:8080

    @internal {
        path /readyz /metrics
        not remote_ip 10.0.0.0/8
    }
    respond @internal 403
}
```

---

## relayd (OpenBSD)

```
table <relay> { 127.0.0.1 }
table <monitors> { 10.0.0.1 10.0.0.2 }

http protocol "rele_http" {
    # Forward real client IP
    header append "X-Forwarded-For" value "$REMOTE_ADDR"

    # Block external access to monitoring endpoints
    block request path "/readyz"
    block request path "/metrics"
    pass  request from <monitors> path "/readyz"
    pass  request from <monitors> path "/metrics"

    pass
}

relay "rele_relay" {
    listen on egress port 443 tls
    protocol "rele_http"
    forward to <relay> port 8080
}
```

The TLS certificate is configured at the `relayd` level; `http-smtp-rele`
does not need to handle TLS when using `relayd`.

---

## HAProxy

```haproxy
frontend rele_front
    bind :443 ssl crt /etc/ssl/relay.pem
    default_backend rele_back
    acl is_internal src 10.0.0.0/8
    acl is_monitor  path /readyz /metrics
    http-request deny if is_monitor !is_internal

backend rele_back
    server relay1 127.0.0.1:8080 check
```

---

## Securing the monitoring endpoints

`/readyz` reveals SMTP reachability. `/metrics` exposes request and SMTP
counters. Both should be restricted to your internal monitoring network.

`/healthz` is a simple liveness probe and may be safely exposed publicly
if your load balancer requires it.
