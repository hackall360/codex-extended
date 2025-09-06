# Python

Generate Python from MUL:

```bash
codex mul --from mul --to python < ../program.mul > main.py
```

The generated `main.py`:

```python
def add(a: int, b: int) -> int:
    return a + b
```

Tooling commands:

- Build: `pip install`
- Test: `pytest`
- Lint: `flake8`
- Run: `python main.py`
