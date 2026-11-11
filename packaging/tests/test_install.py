"""Exercise the real installer in an isolated prefix without compiling Rust."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

INSTALLER = Path(__file__).resolve().parents[2] / "install.sh"
BINARIES = ("magic", "magic-cli", "magic-mcp")


class SourceInstallTest(unittest.TestCase):
    def test_source_build_wins_over_bundled_binaries(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            repo, tools, prefix = root / "repo", root / "tools", root / "prefix"
            repo.mkdir()
            tools.mkdir()
            shutil.copy2(INSTALLER, repo / "install.sh")
            (repo / "bin").mkdir()
            for name in BINARIES:
                binary = repo / "bin" / name
                binary.write_text("#!/bin/sh\n# stale bundle\n")
                binary.chmod(0o755)
            cargo = tools / "cargo"
            cargo.write_text(
                '#!/bin/sh\nset -eu\nmkdir -p target/release\n'
                'for name in magic magic-cli magic-mcp; do\n'
                '  printf "#!/bin/sh\\n# fresh source build\\n" > "target/release/$name"\n'
                '  chmod +x "target/release/$name"\ndone\n'
            )
            cargo.chmod(0o755)
            env = dict(os.environ, PATH=f"{tools}:{os.environ['PATH']}")
            result = subprocess.run(
                ["bash", str(repo / "install.sh"), "--from-source", "--prefix", str(prefix)],
                env=env, capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            for name in BINARIES:
                with self.subTest(binary=name):
                    self.assertEqual(
                        (prefix / "bin" / name).read_bytes(),
                        (repo / "target/release" / name).read_bytes(),
                    )


if __name__ == "__main__":
    unittest.main()
