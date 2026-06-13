"""OmniView module system — cross-file import resolution and loading.

Reuses omnilang.modules.resolver.ModuleResolver and
omnilang.modules.graph.ModuleGraph for path resolution and cycle detection.
Adds OmniView-specific ModuleLoader that handles preprocessing.
"""

from omniview.modules.loader import ModuleLoader

__all__ = ["ModuleLoader"]
