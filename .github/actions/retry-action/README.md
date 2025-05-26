# Retry Action with Network Resilience

A reusable GitHub Actions composite action that provides robust retry mechanisms for network operations and CI/CD tasks prone to transient failures.

## Features

- 🔄 **Configurable retry logic** with customizable attempts and wait times
- ⏱️ **Timeout protection** to prevent hanging operations
- 📊 **Detailed logging** with configurable log levels
- 🛡️ **Error handling** with continue-on-error support
- 📈 **Output tracking** for success status and attempt counts

## Usage

### Basic Usage

```yaml
- name: Run command with retry
  uses: ./.github/actions/retry-action
  with:
    command: 'cargo audit'
    max_attempts: 3
    timeout_minutes: 10
    retry_wait_seconds: 30
```

### Advanced Usage

```yaml
- name: Complex operation with retry
  uses: ./.github/actions/retry-action
  with:
    command: |
      uv sync --all-extras
      uv pip install -e .
      make test-python
    max_attempts: 5
    timeout_minutes: 15
    retry_wait_seconds: 60
    continue_on_error: true
    log_level: debug
    working_directory: ./subproject
```

### Network Operations

```yaml
- name: Download with retry
  uses: ./.github/actions/retry-action
  with:
    command: 'curl -fsSL https://api.github.com/repos/owner/repo/releases/latest'
    max_attempts: 5
    timeout_minutes: 5
    retry_wait_seconds: 10
```

## Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `command` | Command to execute with retry logic | ✅ | - |
| `max_attempts` | Maximum number of retry attempts | ❌ | `3` |
| `timeout_minutes` | Timeout for each attempt in minutes | ❌ | `10` |
| `retry_wait_seconds` | Wait time between retries in seconds | ❌ | `30` |
| `retry_on` | Retry on specific conditions (error, timeout, any) | ❌ | `any` |
| `shell` | Shell to use for command execution | ❌ | `bash` |
| `working_directory` | Working directory for command execution | ❌ | `.` |
| `continue_on_error` | Continue workflow even if all retries fail | ❌ | `false` |
| `log_level` | Logging level (debug, info, warn, error) | ❌ | `info` |

## Outputs

| Output | Description |
|--------|-------------|
| `success` | Whether the command succeeded (`true`/`false`) |
| `attempts_made` | Number of attempts made |
| `final_exit_code` | Final exit code of the command |

## Common Use Cases

### 1. Cargo Operations with Network Dependencies

```yaml
- name: Cargo audit with retry
  uses: ./.github/actions/retry-action
  with:
    command: 'cargo audit --timeout 300'
    max_attempts: 3
    timeout_minutes: 15
    retry_wait_seconds: 60
```

### 2. Python Package Installation

```yaml
- name: Install Python dependencies
  uses: ./.github/actions/retry-action
  with:
    command: |
      uv sync --all-extras
      uv pip install -e .
    max_attempts: 3
    timeout_minutes: 10
```

### 3. API Calls with Rate Limiting

```yaml
- name: GitHub API call
  uses: ./.github/actions/retry-action
  with:
    command: 'gh api repos/${{ github.repository }}/releases'
    max_attempts: 5
    retry_wait_seconds: 120  # Wait longer for rate limits
```

### 4. Test Execution with Flaky Tests

```yaml
- name: Run tests with retry
  uses: ./.github/actions/retry-action
  with:
    command: 'pytest tests/ --maxfail=1'
    max_attempts: 3
    continue_on_error: true
```

## Error Handling

The action handles various types of failures:

- **Network timeouts**: Automatically retries with exponential backoff
- **API rate limits**: Waits between attempts to respect rate limits
- **Transient failures**: Retries operations that might succeed on subsequent attempts
- **Command timeouts**: Uses system timeout to prevent hanging operations

## Logging

The action provides detailed logging at different levels:

- **debug**: Detailed execution information
- **info**: General progress information (default)
- **warn**: Warning messages and retry notifications
- **error**: Error messages only

## Best Practices

1. **Set appropriate timeouts**: Match timeout values to expected operation duration
2. **Use reasonable retry counts**: 3-5 attempts are usually sufficient
3. **Adjust wait times**: Longer waits for rate-limited APIs, shorter for network issues
4. **Enable debug logging**: For troubleshooting complex operations
5. **Use continue_on_error**: For non-critical operations that shouldn't fail the workflow

## Integration with Existing Workflows

This action is designed to be a drop-in replacement for direct command execution in existing workflows. Simply wrap your existing commands with the retry action to add resilience.

### Before
```yaml
- name: Run cargo audit
  run: cargo audit
```

### After
```yaml
- name: Run cargo audit with retry
  uses: ./.github/actions/retry-action
  with:
    command: 'cargo audit'
```

## Troubleshooting

### Common Issues

1. **Command not found**: Ensure the command is available in the runner environment
2. **Permission denied**: Check file permissions and working directory
3. **Timeout too short**: Increase `timeout_minutes` for long-running operations
4. **Too many retries**: Reduce `max_attempts` to avoid excessive resource usage

### Debug Mode

Enable debug logging to get detailed execution information:

```yaml
- uses: ./.github/actions/retry-action
  with:
    command: 'your-command'
    log_level: debug
```

## Contributing

This action is part of the rez-core project. For issues or improvements, please refer to the main project repository.
