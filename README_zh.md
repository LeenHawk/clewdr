<div align="center">
  <img src="./assets/clewdr-logo.svg" alt="ClewdR" height="60">
  
  <p><em>现代化高性能 LLM 代理服务器</em></p>
  
  [![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Xerxes-2/clewdr)
  [![GitHub Release](https://img.shields.io/github/v/release/Xerxes-2/clewdr?style=for-the-badge&logo=github&color=blue)](https://github.com/Xerxes-2/clewdr/releases/latest)
  [![License](https://img.shields.io/github/license/Xerxes-2/clewdr?style=for-the-badge&color=green)](./LICENSE)
  [![Performance](https://img.shields.io/badge/性能-10倍提升-orange?style=for-the-badge)](#性能指标)
  [![Memory](https://img.shields.io/badge/内存-个位数MB-purple?style=for-the-badge)](#技术架构)

  <h3>🌍 语言支持</h3>
  <p>
    <a href="./README.md"><strong>🇺🇸 English</strong></a> |
    <a href="./README_zh.md"><strong>🇨🇳 简体中文</strong></a>
  </p>
</div>

---

## 🎯 **什么是 ClewdR？**

**ClewdR** 是一个生产级的高性能代理服务器，专为 **Claude**（Claude.ai、Claude Code）和 **Google Gemini**（AI Studio、Vertex AI）设计。使用 **Rust** 构建，追求极致性能和最小资源占用，提供企业级可靠性和消费级友好体验。

### 🏆 **为什么选择 ClewdR？**

- **🚄 10倍性能**: 超越脚本语言实现
- **💾 1/10内存**: 生产环境仅占用个位数MB
- **🔧 生产就绪**: 轻松处理每秒上千请求
- **🌐 多平台**: 原生支持 Windows、macOS、Linux、Android

## ✨ **核心功能**

<table>
  <tr>
    <td width="50%">

### 🎨 **全功能Web界面**

- **React驱动的控制台** 实时监控
- **多语言支持** 中英文界面
- **安全认证** 自动生成密码
- **热配置重载** 无需重启服务
- **可视化Cookie和Key管理**

### 🏗️ **企业级架构**

- **Tokio + Axum** 异步运行时最大吞吐量
- **事件驱动设计** 组件解耦
- **Moka缓存技术** 智能失效机制
- **Chrome级别指纹识别** 无缝API访问
- **多线程处理** 最优资源使用

### 🧠 **智能资源管理**

- **智能Cookie轮换** 状态分类
- **API密钥健康监控** 自动故障转移
- **限流保护** 指数退避算法
- **连接池优化** Keep-Alive保持

    </td>
    <td width="50%">

### 🌍 **通用兼容性**

- **静态编译** 单文件部署，零依赖
- **跨平台原生** Windows、macOS、Linux、Android
- **Docker就绪** 优化镜像
- **反向代理友好** 自定义端点支持

### 🚀 **协议支持**

#### **Claude集成**

- ✅ **Claude.ai** Web界面
- ✅ **Claude Code** 专门支持
- ✅ **系统提示缓存** 提升效率
- ✅ **扩展思考模式**
- ✅ **图片附件** 和网页搜索
- ✅ **自定义停止序列**

#### **Google Gemini集成**

- ✅ **AI Studio** 和 **Vertex AI**
- ✅ **OAuth2认证** 企业级
- ✅ **HTTP Keep-Alive** 优化
- ✅ **模型切换** 自动检测

#### **API兼容性**

- ✅ **OpenAI格式** 直接替换
- ✅ **原生格式** Claude和Gemini
- ✅ **流式响应** 实时处理

    </td>
  </tr>

</table>

## 📊 **性能指标**

<div align="center">

| 指标 | ClewdR | 传统代理 |
|------|--------|----------|
| **内存使用** | `<10 MB` | `100-500 MB` |
| **请求/秒** | `1000+` | `100-200` |
| **启动时间** | `<1 秒` | `5-15 秒` |
| **二进制大小** | `~15 MB` | `50-200 MB` |
| **依赖项** | `零依赖` | `Node.js/Python + 库` |

</div>

## 🚀 **快速上手指南**

### **第一步：下载运行**

```bash
# 下载对应平台的最新版本
wget https://github.com/Xerxes-2/clewdr/releases/latest/download/clewdr-[平台]

# 如果需要，解压二进制文件
tar -xzf clewdr-[平台].tar.gz

# 进入目录
cd clewdr-[平台]

# 赋予执行权限 (Linux/macOS)
chmod +x clewdr

# 运行 ClewdR
./clewdr
```

<details>
<summary>📦 <strong>平台下载链接</strong></summary>

| 平台 | 架构 | 下载链接 |
|------|------|----------|
| 🪟 Windows | x64 | [clewdr-windows-x64.exe](https://github.com/Xerxes-2/clewdr/releases/latest) |
| 🐧 Linux | x64 | [clewdr-linux-x64](https://github.com/Xerxes-2/clewdr/releases/latest) |
| 🐧 Linux | ARM64 | [clewdr-linux-arm64](https://github.com/Xerxes-2/clewdr/releases/latest) |
| 🍎 macOS | x64 | [clewdr-macos-x64](https://github.com/Xerxes-2/clewdr/releases/latest) |
| 🍎 macOS | ARM64 (M1/M2) | [clewdr-macos-arm64](https://github.com/Xerxes-2/clewdr/releases/latest) |
| 🤖 Android | ARM64 | [clewdr-android-arm64](https://github.com/Xerxes-2/clewdr/releases/latest) |

</details>

### **第二步：访问Web界面**

1. 🌐 在浏览器中打开 **`http://127.0.0.1:8484`**
2. 🔐 使用控制台显示的 **Web Admin Password** 登录
3. 🎉 欢迎来到 ClewdR 管理界面！

> **💡 专业提示:**
>
> - **忘记密码？** 删除 `clewdr.toml` 文件并重启
> - **Docker用户:** 密码显示在容器日志中
> - **修改密码:** 使用Web界面设置

### **第三步：配置服务**

<table>
<tr>
<td width="50%">

#### 🍃 **Claude 配置**

1. **添加Cookie**: 粘贴您的 Claude.ai 会话cookie
2. **配置代理**: 如需要设置上游代理
3. **测试连接**: 在控制台验证cookie状态

</td>
<td width="50%">

#### 🔹 **Gemini 配置**

1. **添加API密钥**: 输入您的 Google AI Studio 密钥
2. **Gemini CLI**: 上传 gemini-cli 导出的 OAuth 凭证
3. **Vertex AI** (可选): 为企业配置OAuth2
4. **模型选择**: 选择您偏好的模型

</td>
</tr>
</table>

### **第四步：连接应用程序**

ClewdR 提供多种API端点。查看控制台输出获取可用端点：

#### 🔗 **API端点**

```bash
# Claude 端点
Claude Web:    http://127.0.0.1:8484/v1/messages          # 原生格式
Claude OpenAI: http://127.0.0.1:8484/v1/chat/completions  # OpenAI兼容
Claude Code:   http://127.0.0.1:8484/code/v1/messages     # Claude Code

# Gemini 端点
Gemini Native: http://127.0.0.1:8484/v1/v1beta/generateContent    # 原生格式
Gemini CLI:    http://127.0.0.1:8484/gemini-cli/v1/v1beta/generateContent # Gemini CLI（原生）
Gemini OpenAI: http://127.0.0.1:8484/gemini/chat/completions      # OpenAI兼容
Vertex AI:     http://127.0.0.1:8484/v1/vertex/v1beta/            # Vertex AI
```

#### ⚙️ **应用配置示例**

<details>
<summary><strong>SillyTavern 配置</strong></summary>

```json
{
  "api_url": "http://127.0.0.1:8484/v1/chat/completions",
  "api_key": "控制台显示的API密码",
  "model": "claude-3-sonnet-20240229"
}
```

</details>

<details>
<summary><strong>Continue VSCode 扩展</strong></summary>

```json
{
  "models": [
    {
      "title": "Claude via ClewdR",
      "provider": "openai",
      "model": "claude-3-sonnet-20240229",
      "apiBase": "http://127.0.0.1:8484/v1/",
      "apiKey": "控制台显示的API密码"
    }
  ]
}
```

</details>

<details>
<summary><strong>Cursor IDE 配置</strong></summary>

```json
{
  "openaiApiBase": "http://127.0.0.1:8484/v1/",
  "openaiApiKey": "控制台显示的API密码"
}
```

</details>

### **第五步：验证监控**

- ✅ 在Web控制台检查cookie/密钥状态
- ✅ 监控请求日志确认连接成功
- ✅ 使用简单聊天请求测试
- ✅ 享受超快的LLM代理性能！

## 社区资源

**Github 聚合 Wiki**: <https://github.com/Xerxes-2/clewdr/wiki>

## 致谢

- [wreq](https://github.com/0x676e67/wreq) - 用于API访问的出色浏览器指纹识别库。
- [Clewd 修改版](https://github.com/teralomaniac/clewd) - 原始Clewd的修改版本，提供了许多灵感和基础功能。
- [Clove](https://github.com/mirrorange/clove) - 提供Claude Code支持逻辑。
