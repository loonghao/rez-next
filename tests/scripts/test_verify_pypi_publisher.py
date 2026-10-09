"""Check publisher verification without accessing GitHub or PyPI."""

import importlib.util
import io
import json
import urllib.error
from pathlib import Path
from unittest.mock import patch

import pytest

spec = importlib.util.spec_from_file_location(
    "verify_pypi_publisher",
    Path(__file__).resolve().parents[2] / "scripts/verify_pypi_publisher.py",
)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

ENV = {
    "GITHUB_REPOSITORY": "vx-org/rez-next",
    "GITHUB_WORKFLOW_REF": "vx-org/rez-next/.github/workflows/release.yml@refs/heads/main",
    "ACTIONS_ID_TOKEN_REQUEST_URL": "https://example.invalid/id?request=proof",
    "ACTIONS_ID_TOKEN_REQUEST_TOKEN": "request-secret",
}


def test_publisher_verification_exchanges_identity_without_upload_or_token_output():
    responses = [
        io.BytesIO(b'{"value":"identity-secret"}'),
        io.BytesIO(b'{"token":"publisher-secret"}'),
    ]
    with (
        patch.dict(module.os.environ, ENV, clear=True),
        patch.object(
            module.urllib.request, "urlopen", side_effect=responses
        ) as request,
    ):
        receipt = module.verify_publisher()
    assert receipt["authentication"] == "accepted"
    assert receipt["uploaded"] is False
    assert "secret" not in json.dumps(receipt)
    assert request.call_count == 2
    assert request.call_args.args[0].full_url == "https://pypi.org/_/oidc/mint-token"


def test_publisher_verification_rejects_wrong_workflow_before_network_access():
    with (
        patch.dict(
            module.os.environ,
            ENV | {"GITHUB_WORKFLOW_REF": "other/workflow"},
            clear=True,
        ),
        patch.object(module.urllib.request, "urlopen") as request,
        pytest.raises(RuntimeError, match="canonical release workflow"),
    ):
        module.verify_publisher()
    request.assert_not_called()


def test_publisher_rejection_does_not_expose_identity_or_response_body():
    failure = urllib.error.HTTPError(
        "https://secret.invalid", 422, "secret", None, io.BytesIO(b"secret")
    )
    with (
        patch.dict(module.os.environ, ENV, clear=True),
        patch.object(
            module.urllib.request,
            "urlopen",
            side_effect=[io.BytesIO(b'{"value":"identity-secret"}'), failure],
        ),
        pytest.raises(
            RuntimeError, match=r"PyPI publisher rejected authentication \(HTTP 422\)"
        ) as result,
    ):
        module.verify_publisher()
    assert "secret" not in str(result.value)


def test_authentication_only_mode_cannot_start_release_builds():
    workflow = (
        Path(__file__).resolve().parents[2] / ".github/workflows/release.yml"
    ).read_text()
    assert "verify_publisher_only:" in workflow
    assert "type: boolean\n        default: false" in workflow
    assert (
        "verify-publisher:\n    name: Verify PyPI publisher identity\n    if: inputs.verify_publisher_only"
        in workflow
    )
    assert "verify-version:\n    if: ${{ !inputs.verify_publisher_only }}" in workflow
    for job in ("build", "build-wheels"):
        definition = workflow.split(f"  {job}:\n", 1)[1].split("    steps:", 1)[0]
        assert "needs: verify-version" in definition
