"""P0104: String encoding differences across targets."""
from omnilang.analyze.registry import AnalyzeRule
from omnilang.analyze.diagnostic import Severity

class P0104Rule(AnalyzeRule):
    code = "P0104"
    name = "string_encoding"
    severity = Severity.PORT
    category = "portability"
    message = "String encoding behavior differs across target languages (UTF-16 in JS/Dart vs UTF-8 in Rust/Python)"
    help = "For binary string operations, use explicit encoding from omni/string."
    def check(self, ctx): return []  # Stub — needs dataflow analysis
