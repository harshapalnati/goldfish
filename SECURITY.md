# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please report it responsibly.

### How to Report

**DO NOT** create a public GitHub issue for security vulnerabilities.

Instead, please email us at:
- **security@goldfish.dev**

Please include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)
- Your contact information

### Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 1 week
- **Fix timeline**: Depends on severity
  - Critical: 1-2 weeks
  - High: 2-4 weeks
  - Medium: 1-2 months
  - Low: Next release

### Disclosure Policy

We follow responsible disclosure:

1. Reporter submits vulnerability
2. We acknowledge and assess
3. We develop and test fix
4. We release fix
5. We publicly disclose (with reporter's permission)

## Security Best Practices

### For Users

#### API Keys and Credentials

**Never hardcode credentials in your code:**

```rust
// ❌ Bad
let config = PineconeConfig {
    api_key: "sk-abc123...".to_string(),
    ...
};

// ✅ Good
let config = PineconeConfig {
    api_key: std::env::var("PINECONE_API_KEY")?,
    ...
};
```

**Use environment variables or secret management:**

```bash
# .env file (add to .gitignore!)
PINECONE_API_KEY=sk-abc123...
DATABASE_URL=postgres://user:pass@localhost/db
```

#### Data Protection

**Encrypt sensitive data at rest:**

```rust
use agent_memory::Memory;

// For sensitive memories, consider encryption
let sensitive_memory = Memory::new(
    encrypt_sensitive_data("user ssn: 123-45-6789"),
    MemoryType::Identity
);
```

**Use TLS for database connections:**

```rust
// PostgreSQL with TLS
let config = PostgresConfig {
    connection_string: "postgres://user:pass@host/db?sslmode=require".to_string(),
    ...
};
```

#### Access Control

**Implement proper access controls:**

```rust
// Check permissions before saving
if user.has_permission("write_memory") {
    memory.save(&new_memory).await?;
}

// Use session IDs to isolate users
let memory = Memory::new(content, MemoryType::Fact)
    .with_session_id(&user.id);
```

### For Developers

#### Secure Coding Practices

**Input Validation:**

```rust
pub async fn save(&self, memory: &Memory) -> Result<()> {
    // Validate content length
    if memory.content.len() > MAX_CONTENT_LENGTH {
        return Err(Error::Validation("Content too long".to_string()));
    }
    
    // Validate importance range
    if memory.importance < 0.0 || memory.importance > 1.0 {
        return Err(Error::Validation("Invalid importance".to_string()));
    }
    
    // ... save logic
}
```

**SQL Injection Prevention:**

We use parameterized queries exclusively:

```rust
// ✅ Safe - uses parameters
sqlx::query("SELECT * FROM memories WHERE id = $1")
    .bind(id)
    .fetch_one(&pool)
    .await?;

// ❌ Never do this
sqlx::query(&format!("SELECT * FROM memories WHERE id = '{}'", id))
```

**Serialization Safety:**

```rust
// Validate JSON before deserializing
use serde_json::Value;

let value: Value = serde_json::from_str(&json)?;
if let Some(content) = value.get("content") {
    // Validate content
}
```

#### Dependencies

**Regularly update dependencies:**

```bash
# Check for outdated dependencies
cargo outdated

# Update dependencies
cargo update

# Audit for security vulnerabilities
cargo audit
```

**Pin critical dependencies:**

```toml
[dependencies]
# Pin cryptographic libraries
aes-gcm = "=0.10.0"
```

## Known Security Considerations

### Current Limitations

1. **No built-in encryption**: Data is stored unencrypted by default
   - **Mitigation**: Use filesystem encryption or database-level encryption
   - **Future**: Planned encryption at rest feature

2. **No authentication**: The library doesn't implement user authentication
   - **Mitigation**: Implement auth in your application layer
   - **Future**: May add optional auth hooks

3. **Memory content visibility**: All content is searchable
   - **Mitigation**: Encrypt sensitive content before saving
   - **Use**: The `forgotten` flag for soft-delete

### Secure Deployment

#### File Permissions

```bash
# Set restrictive permissions on data directory
chmod 700 ./data
chown $USER:$USER ./data

# For SQLite
chmod 600 ./data/memories.db
```

#### Network Security

```rust
// Only bind to localhost in development
let memory = MemorySystem::new("./data").await?;

// In production, use proper network isolation
// (Handled by your application, not this library)
```

#### Database Security

**PostgreSQL:**
```sql
-- Create dedicated user with minimal permissions
CREATE USER agent_memory WITH PASSWORD 'strong_password';
GRANT CONNECT ON DATABASE mydb TO agent_memory;
GRANT USAGE ON SCHEMA public TO agent_memory;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO agent_memory;
```

**Redis:**
```bash
# Enable AUTH
requirepass your-strong-password

# Bind to specific interface
bind 127.0.0.1

# Disable dangerous commands
rename-command FLUSHDB ""
rename-command FLUSHALL ""
```

## Security Checklist

Before deploying to production:

- [ ] Credentials stored securely (not in code)
- [ ] TLS enabled for database connections
- [ ] File permissions set correctly
- [ ] Input validation implemented
- [ ] Rate limiting configured
- [ ] Logging and monitoring enabled
- [ ] Regular dependency audits scheduled
- [ ] Backup and recovery tested
- [ ] Security headers configured (web apps)
- [ ] CORS configured properly (web apps)

## Vulnerability History

| Date | CVE | Severity | Description | Fixed in |
|------|-----|----------|-------------|----------|
| - | - | - | No vulnerabilities reported yet | - |

## Contact

- Security Team: security@goldfish.dev
- GPG Key: [Download](https://goldfish.dev/security.gpg)
- Response Time: 48 hours

## Acknowledgments

We thank the following security researchers:

- None yet - be the first!

---

Last updated: 2024
