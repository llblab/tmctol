# Production Deployment Guide

## Overview

This guide provides comprehensive instructions for deploying the parachain to production using the modern Omni Node architecture. It covers current deployment patterns, operational best practices, and monitoring strategies for maintaining a robust production parachain.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [System Requirements](#system-requirements)
3. [Build and Deployment](#build-and-deployment)
4. [Network Configuration](#network-configuration)
5. [Security Hardening](#security-hardening)
6. [Monitoring and Operations](#monitoring-and-operations)
7. [Maintenance Procedures](#maintenance-procedures)
8. [Performance Optimization](#performance-optimization)

---

## Architecture Overview

### Omni Node Deployment Model

The parachain uses the Omni Node architecture for streamlined deployment and runtime-focused development:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  polkadot-      │    │  Runtime        │    │  Chain Spec     │
│  omni-node      │──▶│  WASM Blob      │──▶│  Configuration  │
│                 │    │                 │    │                 │
│  (Binary)       │    │  (Logic)        │    │  (Genesis)      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

`Key Benefits`:

- `Simplified Deployment`: Single binary handles all node functionality
- `Runtime Focus`: Only runtime logic needs custom development
- `Automatic Updates`: Node binary updates independently of runtime
- `Reduced Maintenance`: Fewer components to manage and secure

### Network Topology

```
Internet ────▶ Load Balancer ────▶ RPC Nodes ────▶ Collator Nodes
                     │                                    │
                     ▼                                    ▼
              WebSocket/HTTP                        Relay Chain
                   APIs                            (Block Production)
```

---

## System Requirements

### Production Node Specifications

#### Collator Nodes

- `CPU`: 8+ cores (AMD EPYC or Intel Xeon recommended)
- `RAM`: 32GB minimum (64GB recommended)
- `Storage`: 1TB+ NVMe SSD with high IOPS
- `Network`: 1Gbps dedicated bandwidth
- `OS`: Ubuntu 22.04 LTS or RHEL 8+

#### RPC Nodes

- `CPU`: 4+ cores
- `RAM`: 16GB minimum (32GB recommended)
- `Storage`: 500GB+ SSD
- `Network`: 500Mbps bandwidth
- `Load Balancing`: NGINX or similar

#### Archive Nodes (Optional)

- `CPU`: 4+ cores
- `RAM`: 16GB minimum
- `Storage`: 2TB+ for full history
- `Purpose`: Historical data and analytics

### Infrastructure Dependencies

`Required Services`:

- `Monitoring`: Prometheus + Grafana
- `Logging`: ELK Stack or similar
- `Backup`: Automated database snapshots
- `Secrets Management`: HashiCorp Vault or cloud equivalent
- `CI/CD`: Automated deployment pipeline

---

## Build and Deployment

### Environment Setup

```bash
# Install system dependencies
sudo apt update && sudo apt install -y \
    git curl clang cmake pkg-config libssl-dev \
    build-essential llvm libudev-dev make \
    protobuf-compiler

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup default stable
rustup target add wasm32-unknown-unknown

# Download Omni Node binary
LATEST_RELEASE=$(curl -s https://api.github.com/repos/paritytech/polkadot-sdk/releases/latest | grep tag_name | cut -d '"' -f 4)
wget https://github.com/paritytech/polkadot-sdk/releases/download/${LATEST_RELEASE}/polkadot-omni-node
chmod +x polkadot-omni-node
sudo mv polkadot-omni-node /usr/local/bin/

# Verify installation
polkadot-omni-node --version
```

### Runtime Compilation

```bash
# Clone repository
git clone <your-parachain-repository>
cd parachain

# Build optimized runtime
cargo build --release --package tmctol-runtime

# Verify WASM artifact
ls -la target/release/wbuild/tmctol-runtime/
# Should contain: parachain_template_runtime.wasm
```

### Chain Specification Generation

```bash
# Generate production chain specification
polkadot-omni-node chain-spec-builder create \
    --runtime ./target/release/wbuild/tmctol-runtime/parachain_template_runtime.wasm \
    --chain-name "Production Parachain" \
    --chain-id production-parachain \
    --para-id <YOUR_PARA_ID> \
    --relay-chain polkadot \
    named-preset staging

# Customize genesis configuration
# Edit the generated chain spec to set:
# - Initial balances for key accounts
# - Validator/collator session keys
# - Council/technical committee members
# - Treasury initial funding

# Convert to raw format for deployment
polkadot-omni-node chain-spec-builder convert-to-raw \
    ./staging_chain_spec.json \
    --output ./production_chain_spec.json

# Validate chain specification
polkadot-omni-node chain-spec-builder verify \
    ./production_chain_spec.json

# For paseo-local raw (when preset is absent in the binary), use the helper:
# ./scripts/generate-paseo-local-raw.sh
# Outputs: template/chain-specs/paseo-local-raw.json
```

### Docker Deployment

```dockerfile
# Dockerfile
FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY polkadot-omni-node /usr/local/bin/
COPY production_chain_spec.json /etc/parachain/
COPY parachain_template_runtime.wasm /etc/parachain/

EXPOSE 30333 9933 9944 9615

VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/polkadot-omni-node"]
```

```yaml
# docker-compose.yml
version: "3.8"

services:
  collator:
    build: .
    ports:
      - "30333:30333"
      - "9944:9944"
      - "9615:9615"
    volumes:
      - collator_data:/data
      - ./production_chain_spec.json:/etc/parachain/chain_spec.json
    command:
      [
        "--collator",
        "--chain",
        "/etc/parachain/chain_spec.json",
        "--base-path",
        "/data",
        "--port",
        "30333",
        "--rpc-port",
        "9944",
        "--prometheus-port",
        "9615",
        "--rpc-external",
        "--rpc-cors",
        "all",
        "--rpc-methods",
        "safe",
      ]
    restart: unless-stopped

  rpc-node:
    build: .
    ports:
      - "9933:9933"
      - "9945:9944"
    volumes:
      - rpc_data:/data
    command:
      [
        "--chain",
        "/etc/parachain/chain_spec.json",
        "--base-path",
        "/data",
        "--rpc-port",
        "9944",
        "--rpc-external",
        "--rpc-cors",
        "all",
        "--rpc-methods",
        "safe",
        "--sync",
        "fast",
      ]
    restart: unless-stopped

volumes:
  collator_data:
  rpc_data:
```

---

## Network Configuration

### Collator Node Setup

```bash
# Generate session keys
polkadot-omni-node key generate --scheme sr25519 --output-type json > session_keys.json

# Start collator node
polkadot-omni-node \
    --collator \
    --chain ./production_chain_spec.json \
    --base-path /var/lib/parachain \
    --port 30333 \
    --rpc-port 9944 \
    --prometheus-port 9615 \
    --rpc-external \
    --rpc-cors all \
    --rpc-methods safe \
    --name "Production-Collator-01" \
    --telemetry-url "wss://telemetry.polkadot.io/submit/ 0"
```

### RPC Node Configuration

```bash
# Start RPC node
polkadot-omni-node \
    --chain ./production_chain_spec.json \
    --base-path /var/lib/parachain-rpc \
    --port 30334 \
    --rpc-port 9933 \
    --rpc-external \
    --rpc-cors all \
    --rpc-methods safe \
    --sync fast \
    --max-runtime-instances 16 \
    --rpc-max-connections 1000 \
    --name "Production-RPC-01"
```

### Load Balancer Configuration

```nginx
# /etc/nginx/sites-available/parachain-rpc
upstream parachain_rpc {
    least_conn;
    server 10.0.1.10:9933 max_fails=3 fail_timeout=30s;
    server 10.0.1.11:9933 max_fails=3 fail_timeout=30s;
    server 10.0.1.12:9933 max_fails=3 fail_timeout=30s;
}

upstream parachain_ws {
    ip_hash;
    server 10.0.1.10:9944 max_fails=3 fail_timeout=30s;
    server 10.0.1.11:9944 max_fails=3 fail_timeout=30s;
    server 10.0.1.12:9944 max_fails=3 fail_timeout=30s;
}

server {
    listen 80;
    listen 443 ssl http2;
    server_name rpc.yourparachain.com;

    # SSL configuration
    ssl_certificate /etc/ssl/certs/parachain.crt;
    ssl_certificate_key /etc/ssl/private/parachain.key;

    # RPC endpoint
    location / {
        proxy_pass http://parachain_rpc;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_connect_timeout 30s;
        proxy_send_timeout 30s;
        proxy_read_timeout 30s;
    }

    # WebSocket endpoint
    location /ws {
        proxy_pass http://parachain_ws;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_read_timeout 86400;
    }
}
```

---

## Security Hardening

### Network Security

```bash
# Configure firewall (UFW)
sudo ufw default deny incoming
sudo ufw default allow outgoing

# Allow SSH (restrict to management IPs)
sudo ufw allow from <MANAGEMENT_IP> to any port 22

# Allow P2P networking
sudo ufw allow 30333/tcp
sudo ufw allow 30334/tcp

# Allow RPC (only from load balancer)
sudo ufw allow from <LOAD_BALANCER_IP> to any port 9933
sudo ufw allow from <LOAD_BALANCER_IP> to any port 9944

# Allow monitoring
sudo ufw allow from <MONITORING_IP> to any port 9615

sudo ufw enable
```

### Key Management

```bash
# Create secure key storage
sudo mkdir -p /etc/parachain/keys
sudo chmod 700 /etc/parachain/keys

# Generate and store session keys securely
polkadot-omni-node key generate \
    --scheme sr25519 \
    --output-type json \
    --file /etc/parachain/keys/session.json

sudo chmod 600 /etc/parachain/keys/session.json
sudo chown parachain:parachain /etc/parachain/keys/session.json

# Rotate keys regularly (quarterly recommended)
```

### System Hardening

```bash
# Create dedicated user
sudo useradd --system --shell /bin/false --home-dir /var/lib/parachain parachain
sudo mkdir -p /var/lib/parachain
sudo chown parachain:parachain /var/lib/parachain

# Configure systemd service
sudo tee /etc/systemd/system/parachain-collator.service > /dev/null <<EOF
[Unit]
Description=Parachain Collator Node
After=network.target
StartLimitIntervalSec=0

[Service]
Type=simple
Restart=always
RestartSec=5
User=parachain
ExecStart=/usr/local/bin/polkadot-omni-node \\
    --collator \\
    --chain /etc/parachain/production_chain_spec.json \\
    --base-path /var/lib/parachain \\
    --port 30333 \\
    --rpc-port 9944 \\
    --prometheus-port 9615 \\
    --rpc-external \\
    --rpc-cors all \\
    --rpc-methods safe

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable parachain-collator
sudo systemctl start parachain-collator
```

---

## Monitoring and Operations

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: "parachain-collators"
    static_configs:
      - targets:
          - "collator-01:9615"
          - "collator-02:9615"
    scrape_interval: 5s
    metrics_path: /metrics

  - job_name: "parachain-rpc-nodes"
    static_configs:
      - targets:
          - "rpc-01:9615"
          - "rpc-02:9615"
          - "rpc-03:9615"
    scrape_interval: 15s
```

### Key Metrics to Monitor

`Node Health`:

- `substrate_block_height` - Current block height
- `substrate_finalized_height` - Finalized block height
- `substrate_peers` - Connected peer count
- `substrate_sync_extra_justifications` - Sync status

`Performance Metrics`:

- `substrate_block_processing_time` - Block processing duration
- `substrate_transaction_pool_size` - Transaction pool utilization
- `substrate_database_cache_hit_ratio` - Database performance

`System Resources`:

- CPU utilization
- Memory usage
- Disk I/O and space
- Network throughput

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "Parachain Production Monitoring",
    "panels": [
      {
        "title": "Block Height",
        "type": "stat",
        "targets": [
          {
            "expr": "substrate_block_height{job=\"parachain-collators\"}"
          }
        ]
      },
      {
        "title": "Sync Status",
        "type": "stat",
        "targets": [
          {
            "expr": "substrate_block_height - substrate_finalized_height"
          }
        ]
      },
      {
        "title": "Connected Peers",
        "type": "graph",
        "targets": [
          {
            "expr": "substrate_peers{job=\"parachain-collators\"}"
          }
        ]
      }
    ]
  }
}
```

### Log Management

```bash
# Configure log rotation
sudo tee /etc/logrotate.d/parachain > /dev/null <<EOF
/var/log/parachain/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    copytruncate
}
EOF

# Set up centralized logging (rsyslog)
sudo tee -a /etc/rsyslog.conf > /dev/null <<EOF
# Parachain logging
:programname, isequal, "parachain-collator" /var/log/parachain/collator.log
:programname, isequal, "parachain-rpc" /var/log/parachain/rpc.log
& stop
EOF

sudo systemctl restart rsyslog
```

---

## Maintenance Procedures

### Runtime Upgrades

```bash
# Build new runtime
cargo build --release --package tmctol-runtime

# Extract new WASM
cp target/release/wbuild/tmctol-runtime/parachain_template_runtime.wasm \
   /var/backups/runtime-$(date +%Y%m%d).wasm

# Submit upgrade via governance or sudo
# The runtime upgrade will be applied automatically on the next epoch
```

### Database Maintenance

```bash
# Create database backup
sudo systemctl stop parachain-collator
sudo tar -czf /var/backups/parachain-db-$(date +%Y%m%d).tar.gz \
    /var/lib/parachain/chains/

# Database pruning (if needed)
polkadot-omni-node purge-chain \
    --chain /etc/parachain/production_chain_spec.json \
    --base-path /var/lib/parachain \
    --pruning archive

sudo systemctl start parachain-collator
```

### Emergency Procedures

`Node Recovery`:

```bash
# Stop node gracefully
sudo systemctl stop parachain-collator

# Restore from backup if corrupted
sudo rm -rf /var/lib/parachain/chains/
sudo tar -xzf /var/backups/parachain-db-latest.tar.gz -C /

# Start with state recovery
polkadot-omni-node \
    --chain /etc/parachain/production_chain_spec.json \
    --base-path /var/lib/parachain \
    --unsafe-force-node-key-generation \
    --sync fast

sudo systemctl start parachain-collator
```

`Network Partition Response`:

1. Monitor relay chain connectivity
2. Check collator count and health
3. Coordinate with other operators via governance
4. Implement emergency halt if necessary

---

## Performance Optimization

### Hardware Optimization

`Storage Configuration`:

```bash
# Configure optimal mount options for database
sudo tee -a /etc/fstab > /dev/null <<EOF
/dev/nvme0n1 /var/lib/parachain ext4 noatime,nodiratime,data=writeback,barrier=0 0 2
EOF

# Apply kernel optimizations
sudo tee -a /etc/sysctl.conf > /dev/null <<EOF
# Substrate node optimizations
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216
vm.max_map_count = 262144
EOF

sudo sysctl -p
```

### Node Configuration Tuning

```bash
# Optimized collator start command
polkadot-omni-node \
    --collator \
    --chain /etc/parachain/production_chain_spec.json \
    --base-path /var/lib/parachain \
    --port 30333 \
    --rpc-port 9944 \
    --prometheus-port 9615 \
    --rpc-external \
    --rpc-cors all \
    --rpc-methods safe \
    --max-runtime-instances 8 \
    --runtime-cache-size 4 \
    --db-cache 2048 \
    --pruning 1000 \
    --max-heap-pages 4096 \
    --execution-syncing Native \
    --execution-import-block Native \
    --execution-offchain-worker Native \
    --execution-other Native \
    --wasm-execution Compiled
```

### Capacity Planning

`Scale-Out Guidelines`:

- `RPC Nodes`: Add nodes when average response time > 200ms
- `Collators`: Maintain 2-3 active collators minimum
- `Storage`: Plan for 100GB growth per month baseline
- `Bandwidth`: Monitor for 80% utilization on 15-minute averages

`Auto-scaling with Kubernetes`:

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: parachain-rpc-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: parachain-rpc
  minReplicas: 2
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

## Summary

This production deployment guide provides the foundation for operating a robust, scalable parachain using the Omni Node architecture. Key success factors include:

1. `Proper Infrastructure`: Adequate resources and redundancy
2. `Security Focus`: Network hardening and key management
3. `Comprehensive Monitoring`: Proactive issue detection
4. `Operational Procedures`: Documented maintenance and emergency processes
5. `Performance Optimization`: Tuned configuration for production loads

Regular review and updates of these procedures ensure continued operational excellence as the network grows and evolves.
