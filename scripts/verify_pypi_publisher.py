"""Verify the release workflow's PyPI identity without uploading distributions."""

import json
import os
import urllib.error
import urllib.request


def verify_publisher():
    repository = "vx-org/rez-next"
    workflow = f"{repository}/.github/workflows/release.yml@"
    if os.environ.get("GITHUB_REPOSITORY") != repository or not os.environ.get(
        "GITHUB_WORKFLOW_REF", ""
    ).startswith(workflow):
        raise RuntimeError(
            "Publisher verification requires the canonical release workflow"
        )
    oidc_url = os.environ["ACTIONS_ID_TOKEN_REQUEST_URL"]
    separator = "&" if "?" in oidc_url else "?"
    request = urllib.request.Request(
        oidc_url + separator + "audience=pypi",
        headers={
            "Authorization": "Bearer " + os.environ["ACTIONS_ID_TOKEN_REQUEST_TOKEN"]
        },
    )
    phase = "GitHub identity"
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            identity = json.load(response)["value"]
        phase = "PyPI publisher"
        request = urllib.request.Request(
            "https://pypi.org/_/oidc/mint-token",
            data=json.dumps({"token": identity}).encode(),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(request, timeout=30) as response:
            result = json.load(response)
    except urllib.error.HTTPError as error:
        raise RuntimeError(
            f"{phase} rejected authentication (HTTP {error.code})"
        ) from None
    except (OSError, ValueError, KeyError):
        raise RuntimeError(f"{phase} verification failed") from None
    if not isinstance(result.get("token"), str) or not result["token"]:
        raise RuntimeError("PyPI did not return a publisher token")
    return {
        "repository": repository,
        "workflow": "release.yml",
        "authentication": "accepted",
        "uploaded": False,
    }


if __name__ == "__main__":
    print(json.dumps(verify_publisher()))
