"""Shared evidence checks for native ACP skill coexistence probes."""

from verify_acp_integration import require


def correlated_result(results, call_id, expected=None, *, rejected=False):
    """Require exactly one matching native result, with its expected content or error."""
    require(bool(call_id), "tool call ID is missing")
    matches = [row for row in results if row.get("tool_use_id") == call_id]
    require(len(matches) == 1, "missing or ambiguous correlated tool result")
    result = matches[0]
    require(bool(result.get("is_error")) == rejected, "unexpected tool result status")
    if expected is not None:
        require(bool(expected) and expected in str(result.get("content", "")),
                "correlated result lacks expected content")
    return result


def verify_turn(evidence, mounted, rejected):
    expected = {"native", "native_read"}
    if mounted:
        expected.update(("selected", "selected_read", "mcp"))
    calls = evidence["calls"]
    require(set(calls) == expected and all(calls.values()) and len(set(calls.values())) == len(calls),
            "required correlated calls did not complete")
    # A supplier may terminate on refusal after a correlated native failure.
    continuations = len(calls) - int(rejected and evidence.get("terminal_mcp_rejection") is True)
    require(evidence["model_requests"] > continuations, "model did not receive all tool continuations")
    require(evidence["done"] == 1 and evidence["errors"] == 0, "unexpected gateway terminal events")
    require(evidence["mcp_calls"] == int(mounted and not rejected), "unexpected downstream MCP count")
    require(evidence["mcp_decisions"] == (["reject" if rejected else "once"] if mounted else []),
            "missing or unexpected MCP approval decision")
