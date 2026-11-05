"""Executed by the ignored Rust integration test using the real fixed CLI package."""
import json
from importlib.metadata import version

assert version("hermes-agent") == "0.19.0"
from hermes_cli.config import load_config
from hermes_cli.runtime_provider import resolve_runtime_provider
from agent.auxiliary_client import call_llm
from run_agent import AIAgent
from tools.delegate_tool import _resolve_delegation_credentials, _build_child_agent

config = load_config()
runtime = resolve_runtime_provider(requested="custom")
# Exercise every built-in task's real routing and request implementation.
for task, route in config["auxiliary"].items():
    if not isinstance(route, dict):
        continue
    response = call_llm(task=task, messages=[{"role": "user", "content": "offline aux probe"}],
                        temperature=None, max_tokens=64, timeout=3)
    assert response.choices[0].message.content == "HERMES_BOUND_REPLY"

parent = AIAgent(model=config["model"]["default"], provider=runtime["provider"],
                 base_url=runtime["base_url"], api_key=runtime["api_key"],
                 api_mode="chat_completions", quiet_mode=True,
                 enabled_toolsets=[], skip_context_files=True, skip_memory=True)
creds = _resolve_delegation_credentials(config["delegation"], parent)
child = _build_child_agent(0, "offline probe", None, [], creds["model"], 1, 1, parent,
                           override_provider=creds["provider"],
                           override_base_url=creds["base_url"],
                           override_api_key=creds["api_key"],
                           override_api_mode=creds["api_mode"])
result = child.run_conversation("offline delegated probe")
assert "HERMES_BOUND_REPLY" in json.dumps(result)
# An authentication failure must be surfaced, never use the ambient account.
from openai import APIStatusError, AuthenticationError
try:
    call_llm(task="compression", messages=[{"role": "user", "content": "AUTH_FAILURE_PROBE"}],
             max_tokens=64, timeout=3)
except AuthenticationError:
    pass
else:
    raise AssertionError("missing explicit auxiliary authentication failure")
try:
    call_llm(task="web_extract", messages=[{"role": "user", "content": "PAYMENT_FAILURE_PROBE"}],
             max_tokens=64, timeout=3)
except APIStatusError as error:
    assert error.status_code == 402
else:
    raise AssertionError("capacity failure silently switched to another provider")
print("HERMES_AUXILIARY_AND_DELEGATION_OK")
