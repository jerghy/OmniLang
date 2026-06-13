"""
数据连接器脚手架生成器。

从简单配置生成 OmniLang store + action 代码：
  - REST API 连接器
  - GraphQL 查询连接器
  - Firebase Firestore CRUD 连接器

API: POST /api/connector/generate
"""

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Optional
import json
import re


@dataclass
class ConnectorConfig:
    """连接器配置"""
    type: str  # "rest" | "graphql" | "firebase"
    name: str  # Store name
    baseUrl: str = ""  # REST / GraphQL endpoint
    endpoints: list[EndpointDef] = field(default_factory=list)
    # GraphQL-specific
    query: str = ""
    variables: dict = field(default_factory=dict)
    # Firebase-specific
    collection: str = ""
    firebaseConfig: dict = field(default_factory=dict)


@dataclass
class EndpointDef:
    method: str = "GET"  # GET | POST | PUT | DELETE
    path: str = ""       # /users/:id
    action: str = ""     # fetchUsers
    requestType: str = ""  # JSON → struct inference hint
    responseFields: list[FieldDef] = field(default_factory=list)


@dataclass
class FieldDef:
    name: str
    type: str = "String"  # inferred type


def generate_rest_store(config: ConnectorConfig) -> str:
    """从 REST 端点定义生成 OmniLang store 代码"""
    lines: list[str] = []
    store_name = _to_camel(config.name)

    lines.append(f"// Auto-generated REST connector: {store_name}")
    lines.append(f"// Endpoint: {config.baseUrl}")
    lines.append("")

    # Store declaration
    lines.append(f"store {store_name} {{")

    # State signals
    all_response_types: set[str] = set()
    for ep in config.endpoints:
        resp_name = f"{ep.action}Response"
        if ep.responseFields:
            lines.append(f"    // Response type for {ep.action}")
            lines.append(_generate_struct(resp_name, ep.responseFields, indent=4))
            all_response_types.add(resp_name)

    lines.append("")
    lines.append("    // ── State ────────────────────────────────")
    lines.append("    signal loading: bool = false;")
    lines.append("    signal error: String | null = null;")
    lines.append("")

    # Per-endpoint signals for list endpoints
    for ep in config.endpoints:
        if ep.method == "GET" and ":" not in ep.path:
            resp_name = f"{ep.action}Response" if ep.responseFields else "dynamic"
            data_type = resp_name if ep.responseFields else "dynamic"
            lines.append(f"    signal {ep.action}Data: List<{data_type}> | null = null;")
        elif ep.method == "GET":
            resp_name = f"{ep.action}Response" if ep.responseFields else "dynamic"
            data_type = resp_name if ep.responseFields else "dynamic"
            lines.append(f"    signal {ep.action}Data: {data_type} | null = null;")

    lines.append("")

    # Base URL constant
    lines.append(f"    const baseUrl: String = \"{config.baseUrl}\";")
    lines.append("")

    # Actions for each endpoint
    lines.append("    // ── Actions ────────────────────────────────")
    for ep in config.endpoints:
        lines.append("")
        lines.extend(_generate_rest_action(ep, config.baseUrl))

    lines.append("}")
    return "\n".join(lines)


def _generate_rest_action(ep: EndpointDef, base_url: str) -> list[str]:
    """生成单个 REST endpoint 的 store action"""
    lines: list[str] = []
    params = _extract_path_params(ep.path)
    has_body = ep.method in ("POST", "PUT", "PATCH")

    param_list = ", ".join([f"{p}: String" for p in params])
    if has_body:
        if param_list:
            param_list += ", body: dynamic"
        else:
            param_list = "body: dynamic"

    lines.append(f"    action {ep.action}({param_list}) {{")
    lines.append(f"        this.loading = true;")
    lines.append(f"        this.error = null;")

    # Build URL with path params
    if params:
        url_expr = f"\"{base_url}{ep.path}\""
        for p in params:
            url_expr = url_expr.replace(f":{p}", f"${{{p}}}")
        lines.append(f"        const url = `{url_expr}`;")
    else:
        lines.append(f"        const url = \"{base_url}{ep.path}\";")

    # Build fetch options
    lines.append(f"        try {{")
    lines.append(f"            const response = await fetch(url, {{")
    lines.append(f"                method: \"{ep.method}\",")
    if has_body:
        lines.append(f"                headers: {{ \"Content-Type\": \"application/json\" }},")
        lines.append(f"                body: JSON.stringify(body),")
    lines.append(f"            }});")
    lines.append("")
    lines.append(f"            if (!response.ok) {{")
    lines.append(f"                throw Exception(\"{ep.action} failed: \" + response.status);")
    lines.append(f"            }}")
    lines.append("")
    lines.append(f"            const data = await response.json();")

    if ep.method == "GET" and ":" not in ep.path:
        lines.append(f"            this.{ep.action}Data = data;")
    elif ep.method == "GET":
        lines.append(f"            this.{ep.action}Data = data;")
    else:
        lines.append(f"            // Mutation succeeded — reload if needed")
        lines.append(f"            return data;")

    lines.append(f"        }} catch (err) {{")
    lines.append(f"            this.error = err.message;")
    lines.append(f"        }} finally {{")
    lines.append(f"            this.loading = false;")
    lines.append(f"        }}")
    lines.append(f"    }}")
    return lines


def generate_graphql_store(config: ConnectorConfig) -> str:
    """从 GraphQL 查询定义生成 OmniLang store"""
    store_name = _to_camel(config.name)
    lines: list[str] = []

    lines.append(f"// Auto-generated GraphQL connector: {store_name}")
    lines.append(f"// Endpoint: {config.baseUrl}")
    lines.append("")

    lines.append(f"store {store_name} {{")
    lines.append("    signal loading: bool = false;")
    lines.append("    signal error: String | null = null;")
    lines.append("    signal data: dynamic | null = null;")
    lines.append("")
    lines.append(f"    const endpoint: String = \"{config.baseUrl}\";")
    lines.append("")

    # GraphQL query action
    lines.append(f"    action execute(variables: dynamic) {{")
    lines.append(f"        this.loading = true;")
    lines.append(f"        this.error = null;")
    lines.append(f"        try {{")
    lines.append(f"            const response = await fetch(this.endpoint, {{")
    lines.append(f"                method: \"POST\",")
    lines.append(f"                headers: {{ \"Content-Type\": \"application/json\" }},")
    lines.append(f"                body: JSON.stringify({{")
    lines.append(f"                    query: `{_escape_template(config.query)}`,")
    lines.append(f"                    variables,")
    lines.append(f"                }}),")
    lines.append(f"            }});")
    lines.append(f"            if (!response.ok) throw Exception(\"GraphQL error: \" + response.status);")
    lines.append(f"            const result = await response.json();")
    lines.append(f"            if (result.errors) throw Exception(result.errors[0].message);")
    lines.append(f"            this.data = result.data;")
    lines.append(f"        }} catch (err) {{")
    lines.append(f"            this.error = err.message;")
    lines.append(f"        }} finally {{")
    lines.append(f"            this.loading = false;")
    lines.append(f"        }}")
    lines.append(f"    }}")
    lines.append("}")
    return "\n".join(lines)


def generate_firebase_store(config: ConnectorConfig) -> str:
    """生成 Firebase Firestore CRUD store"""
    store_name = _to_camel(config.name)
    coll = config.collection or "items"
    lines: list[str] = []

    lines.append(f"// Auto-generated Firebase Firestore connector: {store_name}")
    lines.append(f"// Collection: {coll}")
    lines.append("")

    lines.append(f"store {store_name} {{")
    lines.append(f"    signal items: List<dynamic> = [];")
    lines.append(f"    signal loading: bool = false;")
    lines.append(f"    signal error: String | null = null;")
    lines.append("")

    # fetchAll
    lines.append("    action fetchAll() {")
    lines.append("        this.loading = true;")
    lines.append("        try {")
    lines.append(f"            const snapshot = await db.collection(\"{coll}\").get();")
    lines.append(f"            this.items = snapshot.docs.map(doc => ({{ id: doc.id, ...doc.data() }}));")
    lines.append("        } catch (err) {")
    lines.append("            this.error = err.message;")
    lines.append("        } finally {")
    lines.append("            this.loading = false;")
    lines.append("        }")
    lines.append("    }")
    lines.append("")

    # create
    lines.append("    action create(data: dynamic) {")
    lines.append("        this.loading = true;")
    lines.append("        try {")
    lines.append(f"            const ref = await db.collection(\"{coll}\").add(data);")
    lines.append(f"            this.items = [...this.items, {{ id: ref.id, ...data }}];")
    lines.append("        } catch (err) {")
    lines.append("            this.error = err.message;")
    lines.append("        } finally {")
    lines.append("            this.loading = false;")
    lines.append("        }")
    lines.append("    }")
    lines.append("")

    # update
    lines.append("    action update(id: String, data: dynamic) {")
    lines.append("        this.loading = true;")
    lines.append("        try {")
    lines.append(f"            await db.collection(\"{coll}\").doc(id).update(data);")
    lines.append(f"            this.items = this.items.map(item => item.id === id ? {{ ...item, ...data }} : item);")
    lines.append("        } catch (err) {")
    lines.append("            this.error = err.message;")
    lines.append("        } finally {")
    lines.append("            this.loading = false;")
    lines.append("        }")
    lines.append("    }")
    lines.append("")

    # delete
    lines.append("    action delete(id: String) {")
    lines.append("        this.loading = true;")
    lines.append("        try {")
    lines.append(f"            await db.collection(\"{coll}\").doc(id).delete();")
    lines.append(f"            this.items = this.items.filter(item => item.id !== id);")
    lines.append("        } catch (err) {")
    lines.append("            this.error = err.message;")
    lines.append("        } finally {")
    lines.append("            this.loading = false;")
    lines.append("        }")
    lines.append("    }")
    lines.append("}")
    return "\n".join(lines)


# ── Helpers ──────────────────────────────────────


def _to_camel(name: str) -> str:
    """kebab-case → PascalCase"""
    return "".join(w.capitalize() for w in re.split(r"[-_\s]+", name) if w)


def _extract_path_params(path: str) -> list[str]:
    """Extract :param names from URL path"""
    return re.findall(r":(\w+)", path)


def _escape_template(s: str) -> str:
    """Escape backticks and ${} for template strings"""
    return s.replace("`", "\\`").replace("${", "\\${")


def _generate_struct(name: str, fields: list[FieldDef], indent: int = 4) -> str:
    """Generate an OmniLang struct from field definitions"""
    pad = " " * indent
    lines = [f"{pad}struct {name} {{"]
    for f in fields:
        lines.append(f"{pad}    {f.name}: {f.type};")
    lines.append(f"{pad}}}")
    return "\n".join(lines)


def infer_type_from_json(json_str: str) -> list[FieldDef]:
    """从 JSON 样本推断字段类型"""
    try:
        data = json.loads(json_str)
    except json.JSONDecodeError:
        return []

    if isinstance(data, list) and data:
        data = data[0]

    if not isinstance(data, dict):
        return []

    fields: list[FieldDef] = []
    for key, val in data.items():
        t = _infer_single_type(val)
        fields.append(FieldDef(name=key, type=t))
    return fields


def _infer_single_type(val) -> str:
    if val is None:
        return "String | null"
    if isinstance(val, bool):
        return "bool"
    if isinstance(val, int):
        return "int"
    if isinstance(val, float):
        return "double"
    if isinstance(val, str):
        return "String"
    if isinstance(val, list):
        if val:
            inner = _infer_single_type(val[0])
            return f"List<{inner}>"
        return "List<dynamic>"
    if isinstance(val, dict):
        return "dynamic"
    return "dynamic"
