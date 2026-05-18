from rez_next.build_plugins import PipFromDownloadBuilder


class SampleWheelBuilder(PipFromDownloadBuilder):
    def __init__(self):
        super().__init__(
            package="dist/pypi_sample-1.0.0-py3-none-any.whl",
            extra_args=["--no-index"],
        )


if __name__ == "__main__":
    SampleWheelBuilder().run()
