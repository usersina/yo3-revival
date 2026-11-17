#!/usr/bin/env python3
"""Published-tag gating and two-release retention, without deleting real GitHub data."""
import copy
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("lifecycle", ROOT / ".github/scripts/release_lifecycle.py")
lifecycle = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(lifecycle)
REPO = "usersina/yo3-revival"
COMMIT = "a" * 40


def digest(number):
    return "sha256:" + f"{number:064x}"


def release(number):
    return {"id": number, "tag_name": f"v0.1.{number}", "draft": False,
            "prerelease": False, "published_at": "2026-11-17T12:00:00Z"}


class Fixture:
    def __init__(self):
        self.repo = REPO
        self.data = {f"/repos/{REPO}": {"owner": {"type": "User"}},
                     f"/repos/{REPO}/releases": [release(n) for n in [9, 1, 10]],
                     f"/repos/{REPO}/git/ref/tags/v0.1.10": {"object": {"type": "commit", "sha": COMMIT}}}
        self.blobs = {}
        self.registry = {}
        self.deleted = []
        for n in [1, 9, 10]:
            asset = 100 + n
            self.data[f"/repos/{REPO}/releases/{n}/assets"] = [{"id": asset, "name": "container-images.json"}]
            manifest = {"version": f"v0.1.{n}", "commit": COMMIT}
            for role in ["server", "dashboard"]:
                image = f"ghcr.io/{REPO}-{role}"
                manifest[role] = f"{image}@{digest(n)}"
                # Real Buildx output: a tagged index, platform image, and untagged provenance manifest.
                self.registry[f"{image}@{digest(n)}"] = {
                    "mediaType": "application/vnd.oci.image.index.v1+json",
                    "manifests": [{"digest": digest(n + 1000)}, {"digest": digest(n + 2000)}]}
                for child in [n + 1000, n + 2000]:
                    self.registry[f"{image}@{digest(child)}"] = {"mediaType": "application/vnd.oci.image.manifest.v1+json"}
            self.blobs[asset] = manifest
        for role in ["server", "dashboard"]:
            path = self.package(role)
            self.data[path] = {"repository": {"full_name": REPO}}
            self.data[path + "/versions"] = [
                {"id": n, "name": digest(n), "metadata": {"container": {"tags": tags}}}
                for root in [1, 9, 10]
                for n, tags in [(root, [f"v0.1.{root}", "sha-" + COMMIT]), (root + 1000, []), (root + 2000, [])]
            ]
            # An orphan from a failed image build should not consume space forever.
            self.data[path + "/versions"].append({"id": 77, "name": digest(77), "metadata": {"container": {"tags": []}}})

    def package(self, role):
        return f"/users/usersina/packages/container/yo3-revival-{role}"

    def get(self, path):
        return copy.deepcopy(self.data[path])

    def list(self, path):
        return self.get(path)

    def asset(self, asset_id):
        return copy.deepcopy(self.blobs[asset_id])

    def delete(self, path):
        self.deleted.append(path)

    def inspect(self, ref):
        return copy.deepcopy(self.registry[ref])


class ReleaseLifecycle(unittest.TestCase):
    def setUp(self):
        output = contextlib.redirect_stdout(io.StringIO())
        output.__enter__()
        self.addCleanup(output.__exit__, None, None, None)

    def test_only_latest_published_release_with_exact_tag_commit_can_build(self):
        api = Fixture()
        lifecycle.verify_release(api, "v0.1.10", COMMIT)
        for field, value in [("draft", True), ("prerelease", True), ("published_at", None)]:
            broken = Fixture()
            broken.data[f"/repos/{REPO}/releases"][2][field] = value
            with self.assertRaises(ValueError):
                lifecycle.verify_release(broken, "v0.1.10", COMMIT)
        for version, commit in [("v0.1.9", COMMIT), ("v0.1.11", COMMIT), ("v0.1.10", "b" * 40)]:
            with self.assertRaises(ValueError):
                lifecycle.verify_release(api, version, commit)
        # An annotated tag resolves to its commit, never the mutable target_commitish branch.
        api.data[f"/repos/{REPO}/git/ref/tags/v0.1.10"]["object"] = {"type": "tag", "sha": "c" * 40}
        api.data[f"/repos/{REPO}/git/tags/{'c' * 40}"] = {"object": {"type": "commit", "sha": COMMIT}}
        lifecycle.verify_release(api, "v0.1.10", COMMIT)
        self.assertEqual(api.deleted, [])

    def test_updater_uses_immediate_previous_version_numerically_not_api_order(self):
        api = Fixture()
        self.assertEqual(lifecycle.previous_release(api, "v0.1.11"), "v0.1.10")
        with self.assertRaises(ValueError):
            lifecycle.previous_release(api, "v0.1.9")
        api.data[f"/repos/{REPO}/releases"] = []
        with self.assertRaises(ValueError):
            lifecycle.previous_release(api, "v0.1.0")

    def test_keeps_two_releases_and_all_referenced_platform_and_provenance_manifests(self):
        api = Fixture()
        lifecycle.prune(api, "v0.1.10", COMMIT, api.inspect)
        expected = [api.package(role) + f"/versions/{n}" for role in ["server", "dashboard"] for n in [1, 1001, 2001, 77]]
        expected.append(f"/repos/{REPO}/releases/1")
        self.assertCountEqual(api.deleted, expected)
        # REST release deletion removes its downloads, never its Git ref/tag.
        self.assertFalse(any("/git/" in path for path in api.deleted))

    def test_drafts_and_non_pipeline_tags_are_not_deleted(self):
        api = Fixture()
        draft = release(20)
        draft["draft"] = True
        api.data[f"/repos/{REPO}/releases"] += [draft, dict(release(30), tag_name="preview")]
        lifecycle.prune(api, "v0.1.10", COMMIT, api.inspect)
        self.assertNotIn(f"/repos/{REPO}/releases/20", api.deleted)
        self.assertNotIn(f"/repos/{REPO}/releases/30", api.deleted)

    def test_legacy_previous_release_without_images_keeps_its_downloads(self):
        api = Fixture()
        api.data[f"/repos/{REPO}/releases/9/assets"] = []
        for role in ["server", "dashboard"]:
            path = api.package(role) + "/versions"
            api.data[path] = [v for v in api.data[path] if v["id"] not in [9, 1009, 2009]]
        lifecycle.prune(api, "v0.1.10", COMMIT, api.inspect)
        self.assertNotIn(f"/repos/{REPO}/releases/9", api.deleted)

    def test_incomplete_retained_images_or_metadata_refuse_all_deletion(self):
        for failure in ["current asset", "legacy image tags", "wrong version", "wrong repository", "missing root", "registry failure", "wrong owner"]:
            with self.subTest(failure=failure):
                api = Fixture()
                if failure == "current asset":
                    api.data[f"/repos/{REPO}/releases/10/assets"] = []
                elif failure == "legacy image tags":
                    api.data[f"/repos/{REPO}/releases/9/assets"] = []
                elif failure == "wrong version":
                    api.blobs[110]["version"] = "v0.1.1"
                elif failure == "wrong repository":
                    api.blobs[109]["dashboard"] = "ghcr.io/unrelated/image@" + digest(9)
                elif failure == "missing root":
                    path = api.package("dashboard") + "/versions"
                    api.data[path] = [v for v in api.data[path] if v["id"] != 10]
                elif failure == "registry failure":
                    del api.registry[f"ghcr.io/{REPO}-dashboard@{digest(2009)}"]
                else:
                    api.data[api.package("dashboard")]["repository"]["full_name"] = "someone/else"
                with self.assertRaises((ValueError, KeyError)):
                    lifecycle.prune(api, "v0.1.10", COMMIT, api.inspect)
                self.assertEqual(api.deleted, [], "All reads/validation must finish before any deletion")

    def test_a_single_release_is_kept_and_repeated_cleanup_is_safe(self):
        api = Fixture()
        lifecycle.prune(api, "v0.1.10", COMMIT, api.inspect)
        for path in api.deleted:
            base, value = path.rsplit("/", 1)
            api.data[base] = [v for v in api.data[base] if v["id"] != int(value)]
        api.deleted.clear()
        lifecycle.prune(api, "v0.1.10", COMMIT, api.inspect)
        self.assertEqual(api.deleted, [])
        api.data[f"/repos/{REPO}/releases"] = [release(10)]
        lifecycle.prune(api, "v0.1.10", COMMIT, api.inspect)
        self.assertNotIn(f"/repos/{REPO}/releases/10", api.deleted)

    def test_failed_package_delete_stops_before_release_deletion(self):
        api = Fixture()
        with patch.object(api, "delete", side_effect=RuntimeError("GitHub refused deletion")):
            with self.assertRaisesRegex(RuntimeError, "GitHub refused"):
                lifecycle.prune(api, "v0.1.10", COMMIT, api.inspect)
        self.assertEqual(api.deleted, [])

    def test_gh_transport_reads_every_page_and_downloads_asset_contents(self):
        api = lifecycle.GitHub(REPO)
        with patch.object(subprocess, "check_output", return_value='[[{"id":1}],[{"id":2}]]') as run:
            self.assertEqual(api.list(f"/repos/{REPO}/releases"), [{"id": 1}, {"id": 2}])
            args = run.call_args.args[0]
            self.assertIn("--paginate", args)
            self.assertIn("--slurp", args)
        with patch.object(subprocess, "check_output", return_value=json.dumps({"version": "v0.1.10"})) as run:
            self.assertEqual(api.asset(110), {"version": "v0.1.10"})
            self.assertIn("Accept: application/octet-stream", run.call_args.args[0])

    def test_cli_exercises_gh_and_registry_transport_without_real_mutations(self):
        # Run the real entry point, but replace only its external executables with a private fixture.
        # In particular, a registry failure on the second package must not delete from the first.
        with tempfile.TemporaryDirectory(prefix="yo3-retention-cli-") as tmp:
            root = Path(tmp)
            api = Fixture()
            state = root / "fixture.json"
            calls = root / "deletes.jsonl"
            for name in ["gh", "docker"]:
                executable = root / name
                executable.write_text(f"#!{sys.executable}\n" + r'''
import json, os, sys
from pathlib import Path
root = Path(__file__).parent
state = json.loads((root / "fixture.json").read_text())
args = sys.argv[1:]
if Path(sys.argv[0]).name == "docker":
    print(json.dumps(state["registry"][args[-1]]))
elif "DELETE" in args:
    with (root / "deletes.jsonl").open("a") as file:
        file.write(json.dumps(args[1]) + "\n")
elif "application/octet-stream" in " ".join(args):
    print(json.dumps(state["blobs"][args[1].rsplit("/", 1)[1]]))
else:
    value = state["data"][args[1].split("?")[0]]
    print(json.dumps([value] if "--slurp" in args else value))
''')
                executable.chmod(0o700)
            env = dict(os.environ, PATH=str(root) + os.pathsep + os.environ["PATH"], GITHUB_REPOSITORY=REPO,
                       GITHUB_STEP_SUMMARY=str(root / "summary.md"))
            def run(command, *args):
                state.write_text(json.dumps({"data": api.data, "blobs": api.blobs, "registry": api.registry}))
                return subprocess.run([sys.executable, SPEC.origin, command, *args], env=env, capture_output=True, text=True)
            self.assertEqual(run("previous", "v0.1.11").stdout.strip(), "v0.1.10")
            self.assertEqual(run("verify", "v0.1.10", COMMIT).returncode, 0)
            ref = f"ghcr.io/{REPO}-dashboard@{digest(2009)}"
            missing = api.registry.pop(ref)
            failed = run("prune", "v0.1.10", COMMIT)
            self.assertNotEqual(failed.returncode, 0)
            self.assertFalse(calls.exists(), failed.stderr)
            api.registry[ref] = missing
            success = run("prune", "v0.1.10", COMMIT)
            self.assertEqual(success.returncode, 0, success.stderr)
            deleted = [json.loads(line) for line in calls.read_text().splitlines()]
            self.assertEqual(len(deleted), 9)
            self.assertEqual(deleted[-1], f"/repos/{REPO}/releases/1")
            self.assertFalse(any(path.endswith("/10") or path.endswith("/9") or "/git/" in path for path in deleted))
            self.assertIn("Git tags preserved", (root / "summary.md").read_text())


if __name__ == "__main__":
    unittest.main()
