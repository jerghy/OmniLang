"""P0103: NaN handling differs across targets."""
from omnilang.analyze.registry import AnalyzeRule
from omnilang.analyze.diagnostic import Severity

class P0103Rule(AnalyzeRule):
    code = "P0103"
    name = "nan_handling"
    severity = Severity.PORT
    category = "portability"
    message = "NaN comparison: NaN != NaN is always true in all targets"
    help = "Use isNaN() check instead of direct comparison with NaN."
    def check(self, ctx):
        return []  # Complex dataflow analysis — stub for now
