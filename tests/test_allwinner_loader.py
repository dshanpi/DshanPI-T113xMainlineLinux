import hashlib
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import allwinner_loader as loader


class LoaderTests(unittest.TestCase):
    manifest = ROOT / "profiles/t113s3-ddr3-spinand-dshanpi-t113s3pro/loader.json"

    def test_profile_and_name(self):
        data, entries = loader.load_manifest(self.manifest)
        self.assertEqual(
            loader.canonical_name(data),
            "t113s3-ddr3-spinand-dshanpi-t113s3pro-loader.bin",
        )
        self.assertEqual([entry["role"] for entry in entries], [item[0] for item in loader.EXPECTED_ENTRIES])

    def test_build_is_vendor_exact_and_verified(self):
        with tempfile.TemporaryDirectory() as temporary:
            image, metadata = loader.build(self.manifest, Path(temporary), check_reproducible=True)
            info = loader.verify_image(self.manifest, image)
            self.assertEqual(info["num_files"], 6)
            self.assertEqual(
                hashlib.sha256(image.read_bytes()).hexdigest(),
                "a2e0e97014e70c7043b634138639444026a4e582f5569bb81c845c6d959d1ea5",
            )
            self.assertTrue(metadata.is_file())

    def test_current_profile_is_ram_only(self):
        data, entries = loader.load_manifest(self.manifest)
        self.assertFalse(data["flash_payload"])
        self.assertTrue(all(entry["flash_to_media"] is False for entry in entries))


if __name__ == "__main__":
    unittest.main()
