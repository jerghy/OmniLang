"""OmniLang module system — import resolution, dependency graph, multi-file loading.

Architecture:
    resolver.py — Cross-platform path resolution (./, ../, std/, @alias/)
    graph.py    — Dependency DAG + Tarjan SCC cycle detection (E0501)
    loader.py   — Multi-file recursive loading + module-level scoping
"""
