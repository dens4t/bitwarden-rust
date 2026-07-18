<div align="center">
  <img src="https://raw.githubusercontent.com/bitwarden/brand/main/icons/icon.svg" width="80" height="80" alt="Bitwarden Icon">
  <h1>Bitwarden-rs</h1>
  <p><strong>Server Bitwarden ringan, cepat, dan hemat sumber daya — ditulis dalam Rust</strong></p>

  <p>
    <img src="https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust" alt="Rust">
    <img src="https://img.shields.io/badge/license-MIT-blue" alt="License">
    <img src="https://img.shields.io/badge/status-stable-brightgreen" alt="Status">
    <img src="https://img.shields.io/badge/RAM-%3E3MB-success" alt="RAM Usage">
  </p>

  <br>
</div>

## ✨ Fitur

- ✅ **Ringan** — Hanya **~3 MB RAM** saat idle, **~5 MB** saat dipakai
- ⚡ **Cepat** — Throughput **~150 req/s**, CPU **~0% idle** (tanpa GC!)
- 🔒 **Kompatibel** — 100% kompatibel dengan Bitwarden clients (web, desktop, mobile, browser extensions)
- 🗄️ **SQLite** — Database single file, tanpa dependency eksternal
- 🔑 **Keamanan** — PBKDF2-SHA256, JWT authentication, 2FA ready
- 📦 **Single Binary** — Deploy cukup copy satu file (`bitwarden-rs`)
- 🐳 **Zero Dependency** — Tidak perlu PHP, Node.js, MySQL, atau Redis

## 📊 Perbandingan Resource

| Metrik | bitwarden-rs (Rust) | Official Bitwarden | bitwarden-go |
|--------|:-------------------:|:------------------:|:------------:|
| **RAM (idle)** | **~3 MB** | ~500 MB+ | ~10 MB |
| **RAM (loaded)** | **~6 MB** | ~1 GB+ | ~15 MB |
| **CPU idle** | **~0%** | ~1-2% | ~0.1% |
| **CPU load (100 req)** | **~1.3%** | - | ~2-3% |
| **Binary size** | **5.5 MB** | ~200 MB+ | ~12 MB |
| **Runtime** | **None (zero-cost)** | .NET / Node.js | Go runtime |

## 🚀 Quick Start

### 1. Unduh Binary

```bash
# Unduh binary terbaru (atau compile sendiri)
# Sementara, compile dari source:
```

### 2. Compile dari Source

```bash
# Prasyarat: Rust 1.70+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Clone & Build
git clone https://github.com/dens4t/bitwarden-rust.git
cd bitwarden-rust
cargo build --release
cp target/release/bitwarden-rs /usr/local/bin/
```

### 3. Jalankan

```bash
# Default (database: bitwarden.db, host: 0.0.0.0, port: 8080)
bitwarden-rs
```

#### ⚙️ Konfigurasi Fleksibel

**Priority:** CLI args > Environment variables > Defaults

**A. Via CLI Arguments**
```bash
# Format: bitwarden-rs [DB_PATH] [BIND_ADDR] [JWT_SECRET]

bitwarden-rs                    # Default: bitwarden.db :8080
bitwarden-rs /data/db           # Custom DB path
bitwarden-rs /data/db 127.0.0.1:9090   # Custom DB + IP:Port
bitwarden-rs /data/db 0.0.0.0:443 key  # Custom semua
```

**B. Via Environment Variables**
```bash
# Host & port terpisah
export HOST=127.0.0.1
export PORT=3000
bitwarden-rs

# Bind address langsung
export BIND_ADDR=0.0.0.0:8080
bitwarden-rs

# Database path
export DB_PATH=/data/bitwarden.db
bitwarden-rs

# JWT Secret (32 karakter random)
export JWT_SECRET=your-secret-key-here
bitwarden-rs

# Semua env var sekaligus
HOST=0.0.0.0 PORT=9090 DB_PATH=/data/db bitwarden-rs
```

**C. Contoh Kombinasi**
```bash
# Hanya ganti port (env var)
PORT=3000 bitwarden-rs

# Hanya bind ke localhost (env var)
HOST=127.0.0.1 bitwarden-rs

# Ganti port via CLI, DB via env
DB_PATH=/data/db bitwarden-rs 0.0.0.0:9090
```

### 4. Konfigurasi Client Bitwarden

Di aplikasi Bitwarden Anda, atur **Self-hosted server** ke:
```
http://your-server-ip:8080
```

## 📖 API Reference

| Method | Endpoint | Auth | Deskripsi |
|--------|----------|:----:|-----------|
| `GET` | `/` | ❌ | Health check |
| `POST` | `/api/accounts/register` | ❌ | Mendaftar akun baru |
| `POST` | `/api/accounts/prelogin` | ❌ | Mendapatkan parameter KDF |
| `POST` | `/identity/connect/token` | ❌ | Login |
| `GET` | `/api/accounts/profile` | ✅ | Mendapatkan profil |
| `GET` | `/api/accounts/keys` | ✅ | Mendapatkan kunci publik/private |
| `POST` | `/api/accounts/keys` | ✅ | Update kunci |
| `GET` | `/api/sync` | ✅ | Sinkronisasi semua data |
| `GET` | `/api/ciphers` | ✅ | Daftar semua cipher |
| `POST` | `/api/ciphers` | ✅ | Buat cipher baru |
| `GET` | `/api/ciphers/{id}` | ✅ | Detail cipher |
| `POST` | `/api/ciphers/{id}` | ✅ | Update cipher |
| `DELETE` | `/api/ciphers/{id}` | ✅ | Hapus cipher |
| `POST` | `/api/ciphers/import` | ✅ | Import cipher |
| `GET` | `/api/folders` | ✅ | Daftar folder |
| `POST` | `/api/folders` | ✅ | Buat folder |
| `POST` | `/api/folders/{id}` | ✅ | Rename folder |
| `DELETE` | `/api/folders/{id}` | ✅ | Hapus folder |
| `GET` | `/api/collections` | ✅ | Daftar koleksi |
| `GET/POST` | `/api/two-factor` | ✅ | Status 2FA |
| `POST` | `/api/two-factor/disable` | ✅ | Nonaktifkan 2FA |

## 🏠 Systemd Service

```bash
cat > /etc/systemd/system/bitwarden-rs.service << 'EOF'
[Unit]
Description=Bitwarden-rs - Lightweight Bitwarden Server
After=network.target

[Service]
Type=simple
Environment=HOST=0.0.0.0
Environment=PORT=8080
Environment=DB_PATH=/var/lib/bitwarden-rs/bitwarden.db
ExecStart=/usr/local/bin/bitwarden-rs
WorkingDirectory=/var/lib/bitwarden-rs
Restart=on-failure
RestartSec=5s
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now bitwarden-rs
```

### Mengganti Port / IP

```bash
# Edit service
systemctl edit bitwarden-rs

# Tambahkan:
[Service]
Environment=HOST=127.0.0.1
Environment=PORT=3000

# Restart
systemctl restart bitwarden-rs
```

## 🛠️ Struktur Proyek

```
src/
├── main.rs           # Entry point & router
├── api/
│   ├── mod.rs        # JWT, middleware, extractor
│   ├── auth.rs       # Register, login, prelogin, 2FA
│   ├── ciphers.rs    # Cipher CRUD
│   ├── folders.rs    # Folder CRUD
│   └── sync.rs       # Sync, profile, keys, collections
├── db/
│   └── mod.rs        # SQLite database (rusqlite)
├── models/
│   └── mod.rs        # Data structures (serde)
└── crypto/
    └── mod.rs        # PBKDF2-SHA256 (ring)
```

## 🔧 Teknologi

| Komponen | Pustaka |
|----------|---------|
| Web Framework | [axum](https://github.com/tokio-rs/axum) 0.8 |
| Runtime | [tokio](https://tokio.rs) |
| Database | [rusqlite](https://github.com/rusqlite/rusqlite) (SQLite bundled) |
| Auth | [jsonwebtoken](https://github.com/Keats/jsonwebtoken) |
| Crypto | [ring](https://github.com/briansmith/ring) (PBKDF2) |
| Serialization | [serde](https://serde.rs) + [serde_json](https://github.com/serde-rs/json) |
| CORS | [tower-http](https://github.com/tower-rs/tower-http) |

## 📈 Performa

Diuji pada CentOS 8 (2 vCPU, 4GB RAM):

```
100 permintaan sync berturut-turut:
  real    0m0.646s
  req/s   ~154

Penggunaan memori:
  idle:   3.0 MB RSS
  aktif:  5.9 MB RSS (dengan 5 cipher + 1 folder)
```

## 🤝 Kontribusi

Kontribusi selalu diterima! Silakan buka _issue_ atau _pull request_.

## ⚠️ Catatan Penting

Proyek ini adalah implementasi **independen** dan **tidak berafiliasi** dengan Bitwarden Inc. Gunakan dengan bijak.

## 📄 Lisensi

MIT License — lihat file [LICENSE](LICENSE) untuk detail.
