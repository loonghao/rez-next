from rez_next.build_plugins import ExtractionBuilder


class VxExtractionBuilder(ExtractionBuilder):
    def __init__(self):
        super().__init__(
            artifact="dist/vx-artifact.zip",
            sha256_file="dist/vx-artifact.sha256",
        )


if __name__ == "__main__":
    VxExtractionBuilder().run()
