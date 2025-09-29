<div align="center">
  <img src="./assets/clewdr-logo.svg" alt="ClewdR" height="60">
  
  <p><em>High-Performance LLM Proxy for the Modern Era</em></p>
  
  [![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Xerxes-2/clewdr)
  [![GitHub Release](https://img.shields.io/github/v/release/Xerxes-2/clewdr?style=for-the-badge&logo=github&color=blue)](https://github.com/Xerxes-2/clewdr/releases/latest)
  [![License](https://img.shields.io/github/license/Xerxes-2/clewdr?style=for-the-badge&color=green)](./LICENSE)
  [![Performance](https://img.shields.io/badge/Performance-10x%20Faster-orange?style=for-the-badge)](#performance-metrics)
  [![Memory](https://img.shields.io/badge/Memory-Single%20Digit%20MB-purple?style=for-the-badge)](#technical-architecture)

  <h3>🌍 Language Support</h3>
  <p>
    <a href="./README.md"><strong>🇺🇸 English</strong></a> |
    <a href="./README_zh.md"><strong>🇨🇳 简体中文</strong></a>
  </p>
</div>

---

## 🎯 **What is ClewdR?**

**ClewdR** is a production-grade, high-performance proxy server engineered specifically for **Claude** (Claude.ai, Claude Code) and **Google Gemini** (AI Studio, Vertex AI). Built with **Rust** for maximum performance and minimal resource usage, it provides enterprise-level reliability with consumer-friendly simplicity.

### 🏆 **Why ClewdR?**

- **🚄 10x Performance**: Outperforms script-language implementations
- **💾 1/10th Memory**: Uses only single-digit MB in production
- **🔧 Production Ready**: Handles thousands of requests per second
- **🌐 Multi-Platform**: Native support for Windows, macOS, Linux, Android

## ✨ **Core Features**

<table>
  <tr>
    <td width="50%">

### 🎨 **Full-Featured Web Interface**

- **React-powered dashboard** with real-time monitoring
- **Multi-language support** (English/Chinese)
- **Secure authentication** with auto-generated passwords
- **Hot configuration reload** without service interruption
- **Visual cookie & key management**

### 🏗️ **Enterprise Architecture**

- **Tokio + Axum** async runtime for maximum throughput
- **Event-driven design** with decoupled components
- **Moka-powered caching** with intelligent invalidation
- **Chrome-level fingerprinting** for seamless API access
- **Multi-threaded processing** with optimal resource usage

### 🧠 **Intelligent Resource Management**

- **Smart cookie rotation** with status classification
- **API key health monitoring** and automatic failover
- **Rate limiting protection** with exponential backoff
- **Connection pooling** with keep-alive optimization

    </td>
    <td width="50%">

### 🌍 **Universal Compatibility**

- **Static compilation** - single binary, zero dependencies
- **Cross-platform native** - Windows, macOS, Linux, Android
- **Docker ready** with optimized images
- **Reverse proxy friendly** with custom endpoint support

### 🚀 **Protocol Support**

#### **Claude Integration**

- ✅ **Claude.ai** web interface
- ✅ **Claude Code** specialized support
- ✅ **System prompt caching** for efficiency
- ✅ **Extended Thinking** mode
- ✅ **Image attachments** & web search
- ✅ **Custom stop sequences**

#### **Google Gemini Integration**

- ✅ **AI Studio** & **Vertex AI**
- ✅ **OAuth2 authentication** for Vertex
- ✅ **HTTP Keep-Alive** optimization
- ✅ **Model switching** with automatic detection

#### **API Compatibility**

- ✅ **OpenAI format** - drop-in replacement
- ✅ **Native formats** - Claude & Gemini
- ✅ **Streaming responses** with real-time processing

    </td>
  </tr>

</table>

## 📊 **Performance Metrics**

<div align="center">

| Metric | ClewdR | Traditional Proxies |
|--------|--------|-------------------|
| **Memory Usage** | `<10 MB` | `100-500 MB` |
| **Requests/sec** | `1000+` | `100-200` |
| **Startup Time** | `<1 second` | `5-15 seconds` |
| **Binary Size** | `~15 MB` | `50-200 MB` |
| **Dependencies** | `Zero` | `Node.js/Python + libs` |

</div>

## 🚀 **Quick Start Guide**

### **Step 1: Download & Run**

```bash
# Download the latest release for your platform
wget https://github.com/Xerxes-2/clewdr/releases/latest/download/clewdr-[platform]

# Extract the binary (if necessary)
tar -xzf clewdr-[platform].tar.gz

# Navigate to the directory
cd clewdr-[platform]

# Make executable (Linux/macOS)
chmod +x clewdr

# Run ClewdR
./clewdr
```

<details>
<summary>📦 <strong>Platform Downloads</strong></summary>

| Platform | Architecture | Download Link |
|----------|-------------|--------------|
| 🪟 Windows | x64 | [clewdr-windows-x64.exe](https://github.com/Xerxes-2/clewdr/releases/latest) |
| 🐧 Linux | x64 | [clewdr-linux-x64](https://github.com/Xerxes-2/clewdr/releases/latest) |
| 🐧 Linux | ARM64 | [clewdr-linux-arm64](https://github.com/Xerxes-2/clewdr/releases/latest) |
| 🍎 macOS | x64 | [clewdr-macos-x64](https://github.com/Xerxes-2/clewdr/releases/latest) |
| 🍎 macOS | ARM64 (M1/M2) | [clewdr-macos-arm64](https://github.com/Xerxes-2/clewdr/releases/latest) |
| 🤖 Android | ARM64 | [clewdr-android-arm64](https://github.com/Xerxes-2/clewdr/releases/latest) |

</details>

### **Step 2: Access Web Interface**

1. 🌐 Open your browser to **`http://127.0.0.1:8484`**
2. 🔐 Use the **Web Admin Password** displayed in the console
3. 🎉 Welcome to ClewdR's management interface!

> **💡 Pro Tips:**
>
> - **Forgot password?** Delete `clewdr.toml` and restart
> - **Docker users:** Password appears in container logs
> - **Change password:** Use the web interface settings

### **Step 3: Configure Your Services**

<table>
<tr>
<td width="50%">

#### 🍃 **Claude Setup**

1. **Add Cookies**: Paste your Claude.ai session cookies
2. **Configure Proxy**: Set upstream proxy if needed
3. **Test Connection**: Verify cookie status in dashboard

</td>
<td width="50%">

#### 🔹 **Gemini Setup**

1. **Add API Keys**: Input your Google AI Studio keys
2. **Gemini CLI**: Upload gemini-cli generated OAuth credentials
3. **Vertex AI** (Optional): Configure OAuth2 for enterprise
4. **Model Selection**: Choose your preferred models

</td>
</tr>
</table>

### **Step 4: Connect Your Applications**

ClewdR provides multiple API endpoints. Check the console output for available endpoints:

#### 🔗 **API Endpoints**

```bash
# Claude Endpoints
Claude Web:    http://127.0.0.1:8484/v1/messages          # Native format
Claude OpenAI: http://127.0.0.1:8484/v1/chat/completions  # OpenAI compatible
Claude Code:   http://127.0.0.1:8484/code/v1/messages     # Claude Code

# Gemini Endpoints  
Gemini Native: http://127.0.0.1:8484/v1/v1beta/generateContent    # Native format
Gemini CLI:    http://127.0.0.1:8484/gemini-cli/v1/v1beta/generateContent # Gemini CLI (native)
Gemini OpenAI: http://127.0.0.1:8484/gemini/chat/completions      # OpenAI compatible
Vertex AI:     http://127.0.0.1:8484/v1/vertex/v1beta/            # Vertex AI
```

#### ⚙️ **Application Configuration Examples**

<details>
<summary><strong>SillyTavern Configuration</strong></summary>

```json
{
  "api_url": "http://127.0.0.1:8484/v1/chat/completions",
  "api_key": "your-api-password-from-console",
  "model": "claude-3-sonnet-20240229"
}
```

</details>

<details>
<summary><strong>Continue VSCode Extension</strong></summary>

```json
{
  "models": [
    {
      "title": "Claude via ClewdR",
      "provider": "openai",
      "model": "claude-3-sonnet-20240229",
      "apiBase": "http://127.0.0.1:8484/v1/",
      "apiKey": "your-api-password-from-console"
    }
  ]
}
```

</details>

<details>
<summary><strong>Cursor IDE Configuration</strong></summary>

```json
{
  "openaiApiBase": "http://127.0.0.1:8484/v1/",
  "openaiApiKey": "your-api-password-from-console"
}
```

</details>

### **Step 5: Verify & Monitor**

- ✅ Check cookie/key status in the web dashboard
- ✅ Monitor request logs for successful connections
- ✅ Test with a simple chat request
- ✅ Enjoy blazing-fast LLM proxy performance!

## 🗃️ **Database Persistence**

ClewdR keeps state in a local `clewdr.toml` file by default. To persist configuration, cookies, and API keys in a database instead, compile the binary with the database feature set and point `persistence.mode` at your driver.

### Enable database features

- Build from source with the matching feature flag: `cargo build --release --no-default-features --features "embed-resource,xdg,db-sqlite"`
- Choose `db-sqlite`, `db-postgres`, or `db-mysql` (they automatically include the base `db` feature)
- Custom Docker images should adjust the `cargo build` step to include the desired `db-*` feature; the published Dockerfile uses file mode by default

### Configure the connection

Add a `persistence` section to `clewdr.toml` (or set the equivalent environment variables):

```toml
[persistence]
mode = "postgres"            # sqlite | postgres | mysql
database_url = "postgres://user:pass@db:5432/clewdr"
```

- **SQLite**: optionally set `sqlite_path = "/var/lib/clewdr/clewdr.db"`; ClewdR expands this to `sqlite:///var/lib/clewdr/clewdr.db?mode=rwc` and creates the parent folder when possible
- **Postgres/MySQL**: `database_url` must be provided (for example `postgres://user:pass@host:5432/db`)
- Environment variable form uses Figment’s double-underscore syntax, e.g.:

  ```bash
  export CLEWDR_PERSISTENCE__MODE=sqlite
  export CLEWDR_PERSISTENCE__SQLITE_PATH=/var/lib/clewdr/clewdr.db
  # or database_url for server backends
  export CLEWDR_PERSISTENCE__DATABASE_URL="postgres://user:pass@db/clewdr"
  ```

### Operational notes & precautions

- On first start ClewdR runs automatic SeaORM migrations (`config`, `cookies`, `keys`, `wasted` tables); ensure the account can create tables and indexes
- API endpoints that write cookies/keys check `GET /api/storage/status`; failed connections surface as `Database storage is unavailable`
- SQLite paths should live on persistent storage (bind mount the directory when running in containers)
- Setting `persistence.mode` without a matching `db-*` build leaves ClewdR in file mode—verify your binary with `clewdr -V` or rebuild with the correct features
- The admin API exposes helpers: `GET /api/storage/status` to inspect health, and authenticated `POST /api/storage/import|export` for file migration

## Community Resources

**Github Aggregated Wiki**: <https://github.com/Xerxes-2/clewdr/wiki>
- [Database persistence guide (中文)](wiki/database.md)

## Acknowledgements

- [wreq](https://github.com/0x676e67/wreq) - Excellent browser fingerprinting library used for API access.
- [Clewd Modified Version](https://github.com/teralomaniac/clewd) - A modified version of the original Clewd, providing many inspirations and foundational features.
- [Clove](https://github.com/mirrorange/clove) - Provides the support logic for Claude Code.
