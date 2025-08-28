#!/usr/bin/env python
import sys
import json
import math
from typing import Any, Dict, List

JSONRPC = "2.0"
MCP_VERSION = "2025-06-18"


def write_response(obj: Dict[str, Any]) -> None:
    sys.stdout.write(json.dumps(obj, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def err(id_val, code: int, message: str) -> None:
    write_response({
        "jsonrpc": JSONRPC,
        "id": id_val,
        "error": {"code": code, "message": message},
    })


def ok(id_val, result: Dict[str, Any]) -> None:
    write_response({"jsonrpc": JSONRPC, "id": id_val, "result": result})


# ---- Tools ----

def tool_schemas() -> List[Dict[str, Any]]:
    return [
        {
            "name": "calculate",
            "title": "Evaluate Math Expression",
            "description": (
                "Safely evaluate a mathematical expression supporting +, -, *, /, **, "
                "parentheses, and common functions: sin, cos, tan, log, exp, sqrt, abs, pow."
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expr": {"type": "string", "description": "Expression to evaluate"}
                },
                "required": ["expr"],
            },
        },
        {
            "name": "quadratic_solve",
            "title": "Solve Quadratic",
            "description": "Solve ax^2 + bx + c = 0 for real or complex roots.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "a": {"type": "number"},
                    "b": {"type": "number"},
                    "c": {"type": "number"},
                },
                "required": ["a", "b", "c"],
            },
        },
        {
            "name": "matrix_det",
            "title": "Matrix Determinant",
            "description": "Compute determinant of a 2x2 or 3x3 matrix.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "matrix": {
                        "type": "array",
                        "items": {"type": "array", "items": {"type": "number"}},
                        "description": "2x2 or 3x3 matrix as nested arrays",
                    }
                },
                "required": ["matrix"],
            },
        },
    ]


# ---- Safe evaluator ----

_SAFE_FUNCS = {
    "sin": math.sin,
    "cos": math.cos,
    "tan": math.tan,
    "log": math.log,
    "exp": math.exp,
    "sqrt": math.sqrt,
    "abs": abs,
    "pow": pow,
}
_SAFE_CONSTS = {"pi": math.pi, "e": math.e, "tau": math.tau}


def _eval_expr(expr: str) -> float:
    import ast

    node = ast.parse(expr, mode="eval")

    def _eval(n):
        if isinstance(n, ast.Expression):
            return _eval(n.body)
        if isinstance(n, ast.Num):  # py<3.8
            return n.n
        if isinstance(n, ast.Constant):
            if isinstance(n.value, (int, float)):
                return n.value
            raise ValueError("constants other than numbers are not allowed")
        if isinstance(n, ast.UnaryOp) and isinstance(n.op, (ast.UAdd, ast.USub)):
            v = _eval(n.operand)
            return +v if isinstance(n.op, ast.UAdd) else -v
        if isinstance(n, ast.BinOp) and isinstance(
            n.op, (ast.Add, ast.Sub, ast.Mult, ast.Div, ast.Pow, ast.Mod)
        ):
            left, right = _eval(n.left), _eval(n.right)
            return {
                ast.Add: left + right,
                ast.Sub: left - right,
                ast.Mult: left * right,
                ast.Div: left / right,
                ast.Pow: left ** right,
                ast.Mod: left % right,
            }[type(n.op)]
        if isinstance(n, ast.Name):
            if n.id in _SAFE_CONSTS:
                return _SAFE_CONSTS[n.id]
            raise ValueError(f"unknown identifier: {n.id}")
        if isinstance(n, ast.Call):
            if not isinstance(n.func, ast.Name):
                raise ValueError("invalid function call")
            fn = n.func.id
            if fn not in _SAFE_FUNCS:
                raise ValueError(f"function not allowed: {fn}")
            args = [_eval(a) for a in n.args]
            if n.keywords:
                raise ValueError("keyword args not allowed")
            return _SAFE_FUNCS[fn](*args)
        raise ValueError("unsupported expression")

    return float(_eval(node))


def call_calculate(arguments: Dict[str, Any]) -> Dict[str, Any]:
    expr = arguments.get("expr", "")
    val = _eval_expr(str(expr))
    content = [{"type": "text", "text": f"result: {val}"}]
    return {"content": content, "structuredContent": {"result": val}}


def call_quadratic(arguments: Dict[str, Any]) -> Dict[str, Any]:
    a = float(arguments.get("a"))
    b = float(arguments.get("b"))
    c = float(arguments.get("c"))
    disc = b * b - 4 * a * c
    if disc >= 0:
        rdisc = math.sqrt(disc)
        x1 = (-b + rdisc) / (2 * a)
        x2 = (-b - rdisc) / (2 * a)
        text = f"roots: {x1}, {x2} (real)"
        roots = [x1, x2]
    else:
        rdisc = math.sqrt(-disc)
        real = -b / (2 * a)
        imag = rdisc / (2 * a)
        x1s = f"{real}+{imag}i"
        x2s = f"{real}-{imag}i"
        text = f"roots: {x1s}, {x2s} (complex)"
        roots = [x1s, x2s]
    return {"content": [{"type": "text", "text": text}], "structuredContent": {"roots": roots}}


def _det2(m):
    return m[0][0] * m[1][1] - m[0][1] * m[1][0]


def _det3(m):
    a, b, c = m[0]
    d, e, f = m[1]
    g, h, i = m[2]
    return a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)


def call_matrix_det(arguments: Dict[str, Any]) -> Dict[str, Any]:
    m = arguments.get("matrix")
    if not isinstance(m, list) or not m or not all(isinstance(r, list) for r in m):
        raise ValueError("matrix must be an array of arrays")
    n = len(m)
    if n not in (2, 3):
        raise ValueError("only 2x2 or 3x3 supported")
    if not all(len(r) == n for r in m):
        raise ValueError("matrix must be square (2x2 or 3x3)")
    if n == 2:
        det = _det2(m)
    else:
        det = _det3(m)
    return {"content": [{"type": "text", "text": f"determinant: {det}"}], "structuredContent": {"det": det}}


def handle_initialize(id_val):
    ok(
        id_val,
        {
            "capabilities": {"tools": {"listChanged": False}},
            "protocolVersion": MCP_VERSION,
            "serverInfo": {"name": "complex_math", "version": "1.0.0", "title": "Complex Math Helper"},
        },
    )


def handle_tools_list(id_val):
    ok(id_val, {"tools": tool_schemas()})


def handle_tools_call(id_val, params: Dict[str, Any]):
    name = params.get("name")
    arguments = params.get("arguments") or {}
    try:
        if name == "calculate":
            res = call_calculate(arguments)
        elif name == "quadratic_solve":
            res = call_quadratic(arguments)
        elif name == "matrix_det":
            res = call_matrix_det(arguments)
        else:
            err(id_val, -32601, f"Unknown tool: {name}")
            return
        # Shape as CallToolResult
        result = {"content": res["content"], "structuredContent": res.get("structuredContent")}
        ok(id_val, result)
    except Exception as e:
        write_response(
            {
                "jsonrpc": JSONRPC,
                "id": id_val,
                "result": {"content": [{"type": "text", "text": f"error: {e}"}], "isError": True},
            }
        )


def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except Exception:
            # Ignore malformed
            continue
        method = msg.get("method")
        id_val = msg.get("id")
        if method == "initialize":
            handle_initialize(id_val)
        elif method == "tools/list":
            handle_tools_list(id_val)
        elif method == "tools/call":
            params = msg.get("params") or {}
            handle_tools_call(id_val, params)
        else:
            # Respond with JSON-RPC method not found
            err(id_val, -32601, f"Unknown method: {method}")


if __name__ == "__main__":
    main()

