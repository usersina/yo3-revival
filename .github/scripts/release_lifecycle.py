#!/usr/bin/env python3
"""Gate image builds on a published tag; retain the newest two stable releases after deployment.
Releases before v0.1.0 (the alpha) are never deleted.

Commands: previous VERSION | verify VERSION COMMIT | prune VERSION COMMIT.
Uses the job's GH_TOKEN and Docker login. Git refs/tags are never deleted.
"""
import json
import os
import re
import subprocess
import sys

VERSION = re.compile(r"v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)")
DIGEST = re.compile(r"sha256:[0-9a-f]{64}")
COMMIT = re.compile(r"[0-9a-f]{40}")
INDEX_TYPES = {"application/vnd.oci.image.index.v1+json", "application/vnd.docker.distribution.manifest.list.v2+json"}
FIRST_BETA = (0, 1, 0)
IMAGE_TYPES = {"application/vnd.oci.image.manifest.v1+json", "application/vnd.docker.distribution.manifest.v2+json"}


class GitHub:
    def __init__(self, repo):
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo):
            raise ValueError("Expected an owner/repository")
        self.repo = repo.lower()

    def request(self, path, *args):
        return subprocess.check_output(["gh", "api", path, *args], text=True, timeout=120)

    def get(self, path):
        return json.loads(self.request(path))

    def list(self, path):
        pages = json.loads(self.request(path + "?per_page=100", "--paginate", "--slurp"))
        if not isinstance(pages, list) or not all(isinstance(page, list) for page in pages):
            raise ValueError(f"Expected paginated lists: {path}")
        return [item for page in pages for item in page]

    def asset(self, asset_id):
        return json.loads(self.request(f"/repos/{self.repo}/releases/assets/{int(asset_id)}",
                                       "-H", "Accept: application/octet-stream"))

    def delete(self, path):
        self.request(path, "--method", "DELETE")


def version_key(version):
    match = VERSION.fullmatch(version)
    if not match:
        raise ValueError(f"Expected a stable vX.Y.Z release: {version}")
    return tuple(map(int, match.groups()))


def published_releases(api):
    return sorted((r for r in api.list(f"/repos/{api.repo}/releases")
                   if not r["draft"] and not r["prerelease"] and r.get("published_at")
                   and VERSION.fullmatch(r["tag_name"])), key=lambda r: version_key(r["tag_name"]), reverse=True)


def previous_release(api, version):
    releases = published_releases(api)
    if not releases or version_key(releases[0]["tag_name"]) >= version_key(version):
        raise ValueError("A new release must follow an existing published version for the updater check")
    return releases[0]["tag_name"]


def verify_release(api, version, commit):
    version_key(version)
    if not COMMIT.fullmatch(commit):
        raise ValueError("Expected the full source commit")
    releases = published_releases(api)
    if not releases or releases[0]["tag_name"] != version:
        raise ValueError("Images and cleanup require the newest successfully published stable release")
    obj = api.get(f"/repos/{api.repo}/git/ref/tags/{version}")["object"]
    for _ in range(8):
        if obj["type"] == "commit" and obj["sha"] == commit:
            return releases
        if obj["type"] != "tag" or not COMMIT.fullmatch(obj["sha"]):
            break
        obj = api.get(f"/repos/{api.repo}/git/tags/{obj['sha']}")["object"]
    raise ValueError("The published release tag does not resolve to the build's source commit")


def inspect_manifest(ref):
    return json.loads(subprocess.check_output(["docker", "buildx", "imagetools", "inspect", "--raw", ref],
                                            text=True, timeout=120))


def manifest_tree(image, root, inspect):
    """Keep referenced platform/attestation manifests too; deleting 'untagged' blindly breaks images."""
    retained = set()

    def visit(digest):
        if not DIGEST.fullmatch(digest):
            raise ValueError("Invalid image manifest digest")
        if digest in retained:
            return
        if len(retained) >= 100:
            raise ValueError("Unexpectedly large image manifest tree")
        retained.add(digest)
        manifest = inspect(f"{image}@{digest}")
        media = manifest["mediaType"]
        if media in INDEX_TYPES:
            if not manifest.get("manifests"):
                raise ValueError("Empty image index")
            for child in manifest["manifests"]:
                visit(child["digest"])
        elif media not in IMAGE_TYPES:
            raise ValueError(f"Unsupported image manifest: {media}")

    visit(root)
    return retained


def retention_plan(api, version, commit, inspect):
    releases = verify_release(api, version, commit)
    keep = releases[:2]
    manifests = {}
    for release in keep:
        assets = api.list(f"/repos/{api.repo}/releases/{release['id']}/assets")
        images = [a for a in assets if a["name"] == "container-images.json"]
        if len(images) > 1 or (release["tag_name"] == version and not images):
            raise ValueError("The current release must have complete container-images.json metadata")
        if images:
            manifest = api.asset(images[0]["id"])
            if manifest["version"] != release["tag_name"] or not COMMIT.fullmatch(manifest["commit"]):
                raise ValueError("Image metadata disagrees with the release")
            if release["tag_name"] == version and manifest["commit"] != commit:
                raise ValueError("Image metadata disagrees with the deployed commit")
            manifests[release["tag_name"]] = manifest
    owner, name = api.repo.split("/")
    owner_type = api.get(f"/repos/{api.repo}")["owner"]["type"]
    if owner_type not in {"User", "Organization"}:
        raise ValueError("Unknown repository owner type")
    scope = "users" if owner_type == "User" else "orgs"
    deletes = []
    for role in ["server", "dashboard"]:
        package = f"{name}-{role}"
        path = f"/{scope}/{owner}/packages/container/{package}"
        metadata = api.get(path)
        if metadata.get("repository", {}).get("full_name", "").lower() != api.repo:
            raise ValueError(f"Refusing to prune a package not linked to {api.repo}: {package}")
        versions = api.list(path + "/versions")
        image = f"ghcr.io/{api.repo}-{role}"
        roots, retained = set(), set()
        for release in keep:
            tag = release["tag_name"]
            if tag not in manifests:
                # Old archive-only releases are valid, but a missing manifest for existing images is not.
                if any(tag in v["metadata"]["container"]["tags"] for v in versions):
                    raise ValueError(f"Missing image metadata for {tag}")
                continue
            ref = manifests[tag][role]
            prefix = image + "@"
            if not ref.startswith(prefix) or not DIGEST.fullmatch(ref.removeprefix(prefix)):
                raise ValueError(f"Unexpected {role} image reference for {tag}")
            root = ref.removeprefix(prefix)
            roots.add(root)
            retained.update(manifest_tree(image, root, inspect))
        if not roots.issubset({v["name"] for v in versions}):
            raise ValueError(f"Retained image versions are missing from GitHub: {package}")
        for item in versions:
            if not DIGEST.fullmatch(item["name"]) or not isinstance(item["id"], int) or item["id"] <= 0:
                raise ValueError("Invalid container package version")
            if item["name"] not in retained:
                deletes.append(f"{path}/versions/{item['id']}")
    # The alpha (v0.0.x, before the first beta v0.1.0) is kept for good: only beta releases past the newest two go.
    deletes.extend(f"/repos/{api.repo}/releases/{r['id']}" for r in releases[2:] if version_key(r["tag_name"]) >= FIRST_BETA)
    return [r["tag_name"] for r in keep], deletes


def prune(api, version, commit, inspect=inspect_manifest):
    # Complete all reads before deleting anything. Errors fail CI; they never roll back a healthy deployment.
    keep, deletes = retention_plan(api, version, commit, inspect)
    print("Retaining releases: " + ", ".join(keep), flush=True)
    for path in deletes:
        print("Deleting " + path, flush=True)
        api.delete(path)
    message = f"Retained {', '.join(keep)} and their image manifest trees; removed {len(deletes)} old releases/package versions. Git tags preserved."
    print(message)
    if summary := os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(summary, "a") as file:
            file.write(message + "\n")


def main():
    api = GitHub(os.environ["GITHUB_REPOSITORY"])
    args = sys.argv[1:]
    if len(args) == 2 and args[0] == "previous":
        print(previous_release(api, args[1]))
    elif len(args) == 3 and args[0] == "verify":
        verify_release(api, args[1], args[2])
        print(f"Verified published {args[1]} at {args[2]}")
    elif len(args) == 3 and args[0] == "prune":
        prune(api, args[1], args[2])
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
