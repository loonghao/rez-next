from importlib.metadata import metadata


def test_python_support_starts_at_39():
    assert metadata("rez-next")["Requires-Python"] == ">=3.9"
