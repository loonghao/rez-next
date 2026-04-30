# GEMINI.md - Google Gemini Guidance for rez-next

> **Important**: This file references [AGENTS.md](./AGENTS.md) for core project information. Read that file first.

## For Google Gemini Agents

When working with the rez-next project, follow this guidance:

### 1. Start Here
Read [AGENTS.md](./AGENTS.md) to understand:
- Project structure and architecture
- Available Python modules and their Rez equivalents
- Development workflow (build, test, lint)
- Common tasks and how to approach them

### 2. Key Principles
- **Progressive disclosure**: AGENTS.md → llms.txt → llms-full.txt → docs/
- **Check coverage**: Not all Rez APIs are implemented; verify before suggesting code
- **Use vx**: Tool management via `vx` (see `vx.toml`)
- **Run CI early**: `vx just ci` before submitting changes

### 3. Python Code Patterns

```python
# ✅ Correct: Use rez_next as drop-in
import rez_next as rez
from rez_next.packages_ import get_latest_package

# ❌ Avoid: Direct rez imports (won't work with rez-next)
import rez
```

### 4. Rust Code Patterns

```rust
// ✅ Correct: Use the crate structure
use rez_next_version::Version;
use rez_next_package::Package;

// ❌ Avoid: Direct std types when crate types exist
```

### 5. Documentation Updates

When updating documentation:
1. Update AGENTS.md if project structure changes
2. Update llms.txt if AI-facing API changes
3. Update llms-full.txt for detailed API changes
4. Update docs/*.md for human-facing changes
5. Update README.md for user-facing changes

### 6. Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):
```
feat: add new solver heuristic
fix(solver): handle cyclic dependencies
docs: update AGENTS.md with new module
chore: upgrade dependencies
```

### 7. Testing Strategy

```bash
# Always run tests before submitting
vx just test

# Run specific tests
vx cargo test --package rez-next-solver

# Run Python tests
maturin develop --release && pytest
```

### 8. Common Pitfalls

❌ **Assuming full Rez compatibility**
- Check `docs/python-integration.md` for coverage

❌ **Not running CI before PR**
- Run `vx just ci` locally first

❌ **Forgetting to update docs**
- If you add a feature, document it in llms-full.txt and docs/

❌ **Breaking Python API**
- This is a pre-1.0 project; be careful with breaking changes

### 9. Performance Considerations

- Use `vx just bench` to benchmark changes
- Check `docs/performance.md` for profiling guide
- Consider caching (see `rez-next-cache` crate)
- Rust is fast, but Python bindings have overhead

### 10. Getting Help

- **Project questions**: Read AGENTS.md first
- **API questions**: Check llms-full.txt
- **Development questions**: Read docs/contributing.md
- **Bug reports**: https://github.com/loonghao/rez-next/issues

---

## Quick Reference

| Task | Command |
|------|---------|
| Build | `vx just build` |
| Test | `vx just test` |
| Lint | `vx just lint` |
| Format | `vx just fmt` |
| Full CI | `vx just ci` |
| Benchmarks | `vx just bench` |
| Docs check | `vx just doc-check` |

## Important Files

| File | Purpose |
|------|---------|
| `AGENTS.md` | **Read this first** - project map |
| `llms.txt` | AI-friendly usage index |
| `llms-full.txt` | Complete API reference |
| `README.md` | User-facing overview |
| `docs/` | Detailed guides |
| `vx.toml` | Tool versions |
| `justfile` | Common tasks |

## Gemini-Specific Notes

### Multimodal Understanding
If you're analyzing code, screenshots, or diagrams:
- Code structure: Check `crates/` directory tree in AGENTS.md
- Performance graphs: See `docs/benchmark_guide.md`
- Architecture diagrams: May be added to `docs/` in future

### Long Context Usage
This project has extensive documentation. Use your long context window to:
1. Read AGENTS.md fully before starting
2. Reference llms-full.txt for API details
3. Keep docs/contributing.md open for workflow questions

### Code Generation
When generating code for this project:
- Follow Rust idioms (see `crates/*/src/*.rs` for examples)
- Use PyO3 patterns (see `crates/rez-next-python/src/*.rs`)
- Match existing code style (enforced by `vx just lint` and `vx just fmt`)

---

**Remember**: When in doubt, read AGENTS.md. It's designed to be your progressive disclosure map for the entire project.
