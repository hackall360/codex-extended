# JavaScript

Generate JavaScript from MUL:

```bash
codex mul --from mul --to javascript < ../program.mul > main.js
```

The generated `main.js`:

```javascript
function add(a, b) {
  return a + b;
}
```

Tooling commands:

- Build: `npm install`
- Test: `npm test`
- Lint: `npm run lint`
- Run: `node main.js`
