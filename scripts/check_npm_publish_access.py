#!/usr/bin/env python3
"""Validate GitHub OIDC publishing access without publishing or logging tokens."""

import json
import os
import urllib.error
import urllib.parse
import urllib.request

PACKAGES = ("shiju", "shiju-acp", "shiju-desktop", "shiju-webview")


def check_access():
    audience = urllib.parse.quote("npm:registry.npmjs.org", safe="")
    url = os.environ["ACTIONS_ID_TOKEN_REQUEST_URL"] + "&audience=" + audience
    request = urllib.request.Request(url, headers={
        "Authorization": "Bearer " + os.environ["ACTIONS_ID_TOKEN_REQUEST_TOKEN"],
    })
    with urllib.request.urlopen(request, timeout=30) as response:
        identity = json.load(response)["value"]
    for name in PACKAGES:
        package = "@shiju-ai/" + name
        url = "https://registry.npmjs.org/-/npm/v1/oidc/token/exchange/package/"
        url += urllib.parse.quote(package, safe="")
        request = urllib.request.Request(url, data=b"", method="POST", headers={
            "Authorization": "Bearer " + identity,
        })
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                if not json.load(response).get("token"):
                    raise SystemExit(f"npm OIDC exchange returned no token for {package}")
        except urllib.error.HTTPError as error:
            error.close()
            raise SystemExit(f"npm OIDC access denied for {package} (HTTP {error.code})") from None
        print(f"npm OIDC access verified: {package}")


if __name__ == "__main__":
    check_access()
