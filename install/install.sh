#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail
if ! command -v python3 >/dev/null 2>&1; then
  echo 'SETUP DEPENDENCY MISSING: Python 3.12+ required. Ubuntu 24.04: sudo apt-get install python3 ca-certificates curl' >&2
  exit 2
fi
exec python3 - "$0" "$@" <<'PY'
"""Released-package setup. No provider authentication or service mutations."""
import argparse
import ctypes
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import platform
import re
import shutil
import shlex
import stat
import subprocess
import sys
import tarfile
import tempfile
import urllib.error
import urllib.request

VERSION = '0.1.0'
SEED_GH = '2.103.3'
SEED_SHA = '4c806978ffce1cec70853302a56d70ac6da4f19bae7adca4e1a8ab00633dd91d'
# namespace: repository basename, package, libc, tag prefix, signer workflow
RECIPES = {
    'cli': ('cli', 'zix-cli', 'gnu', 'v', 'release.yml'),
    'domain': ('zixcel-domain', 'zixcel-domain-cli', 'musl', 'native/domain/v', 'release-native.yml'),
    'cloudflare': ('zixcel-cloudflare', 'zixcel-cloudflare-cli', 'musl', 'native/cloudflare/v', 'release-native.yml'),
    'microsoft': ('zixcel-microsoft', 'zixcel-microsoft-cli', 'gnu', 'native/microsoft/v', 'release-native.yml'),
    'github': ('zixcel-github', 'zixcel-github-cli', 'musl', 'native/github/v', 'release-native.yml'),
    'docker': ('zixcel-container-registry', 'zixcel-container-registry', 'gnu', 'v', 'release.yml'),
    'azure': ('zixcel-azure', 'zixcel-azure-cli', 'musl', 'native/azure/v', 'release-native.yml'),
    'apple': ('zixcel-apple', 'zixcel-apple-cli', 'musl', 'native/apple/v', 'release-native.yml'),
    'google': ('zixcel-google', 'zixcel-google-cli', 'musl', 'google/v', 'release-google.yml'),
    'wordpress': ('zixcel-wordpress', 'zixcel-wordpress', 'gnu', 'v', 'release.yml'),
}


def emit(state, **fields):
    print(json.dumps({'schema': 'zix-setup/v1', 'state': state, **fields}), flush=True)


def require(ok, reason):
    if not ok:
        raise ValueError(reason)


def semver(value):
    require(bool(re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', value)), 'invalid_version')
    return tuple(map(int, value.split('.')))


def digest(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def private_dir(path, private=True):
    # Reject symlink/writable ancestors before creating anything below them.
    for part in reversed([path, *path.parents]):
        if part.exists() or part.is_symlink():
            m = part.lstat()
            require(stat.S_ISDIR(m.st_mode), 'unsafe_directory')
            require(not m.st_mode & 0o022 or (m.st_uid == 0 and m.st_mode & stat.S_ISVTX), 'writable_ancestor')
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    m = path.stat()
    require(m.st_uid == os.getuid() and (not private or not m.st_mode & 0o077), 'unsafe_install_directory')


def fetch(url, limit):
    require(url.startswith('https://'), 'https_required')
    request = urllib.request.Request(url, headers={'User-Agent': 'zix-setup/' + VERSION, 'Accept': 'application/vnd.github+json' if url.startswith('https://api.github.com/') else '*/*'})
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            require(response.url.startswith('https://'), 'https_redirect_required')
            data = response.read(limit + 1)
    except urllib.error.HTTPError as e:
        if e.code in (403, 429):
            raise ValueError('DOWNLOAD ACCESS/RATE LIMIT: stop; inspect GitHub response or wait for rate-limit reset; no provider permission change required') from None
        raise ValueError('download_http_' + str(e.code)) from None
    require(len(data) <= limit, 'download_too_large')
    return data


def api(path):
    return json.loads(fetch('https://api.github.com/' + path, 8 * 1024 * 1024))


def checksums(text):
    result = {}
    for line in text.splitlines():
        match = re.fullmatch(r'([a-fA-F0-9]{64}) [ *](.+)', line)
        require(match is not None, 'invalid_checksum_index')
        sha, name = match.groups()
        path = PurePosixPath(name)
        require(not path.is_absolute() and '..' not in path.parts and name not in result, 'unsafe_checksum_path')
        result[name] = sha.lower()
    return result


def unpack(archive, destination, *, materialize_links=False):
    destination.mkdir(mode=0o700)
    with tarfile.open(archive, 'r:gz') as t:
        members = t.getmembers()
        require(0 < len(members) <= 10000, 'archive_entry_limit')
        names = set()
        total = 0
        for m in members:
            p = PurePosixPath(m.name)
            require(p.parts and not p.is_absolute() and '..' not in p.parts and m.name not in names, 'unsafe_archive_path')
            require(m.isfile() or m.isdir() or (materialize_links and m.issym()), 'archive_links_forbidden')
            total += m.size
            require(total <= 256 * 1024 * 1024 and m.size <= 128 * 1024 * 1024, 'archive_size_limit')
            names.add(m.name)
        # Runtime archives may contain versioned .so symlinks. Resolve only
        # same-directory links to regular archive members, then copy the bytes;
        # no links are installed or used as extraction ancestors.
        by_name = {str(PurePosixPath(m.name)): m for m in members}
        resolved = {}
        for m in members:
            if not m.issym():
                continue
            cursor = m
            seen = set()
            while cursor.issym():
                require(cursor.name not in seen, 'archive_link_cycle')
                seen.add(cursor.name)
                require(bool(cursor.linkname) and '/' not in cursor.linkname and cursor.linkname not in ('.', '..'), 'unsafe_archive_link')
                key = str(PurePosixPath(cursor.name).parent / cursor.linkname)
                require(key in by_name, 'missing_archive_link_target')
                cursor = by_name[key]
            require(cursor.isfile(), 'archive_link_target_not_file')
            total += cursor.size
            require(total <= 256 * 1024 * 1024, 'archive_size_limit')
            resolved[m.name] = cursor.name
        t.extractall(destination, members=[m for m in members if not m.issym()], filter='data')
        for name, target in resolved.items():
            shutil.copyfile(destination / target, destination / name)
            (destination / name).chmod((destination / target).stat().st_mode & 0o777)
    roots = list(destination.iterdir())
    require(len(roots) == 1 and roots[0].is_dir(), 'invalid_archive_root')
    return roots[0]


def verify_contents(package):
    sums = checksums((package / 'CONTENTS.sha256').read_text())
    actual = {str(p.relative_to(package)) for p in package.rglob('*') if p.is_file()}
    require(actual == set(sums) | {'CONTENTS.sha256'}, 'unexpected_package_files')
    for name, sha in sums.items():
        require(digest(package / name) == sha, 'package_contents_mismatch')


def execute(argv, *, timeout=180, config=None):
    # No provider secrets, GH tokens or saved GitHub authentication are inherited.
    env = {k: os.environ[k] for k in ['HOME', 'XDG_CONFIG_HOME', 'HTTPS_PROXY', 'HTTP_PROXY', 'NO_PROXY', 'SSL_CERT_FILE', 'SSL_CERT_DIR'] if k in os.environ}
    env.update(PATH='/usr/bin:/bin', GH_PROMPT_DISABLED='1')
    if config:
        env['GH_CONFIG_DIR'] = str(config)
    r = subprocess.run([str(a) for a in argv], env=env, capture_output=True, timeout=timeout)
    # Raw helper output can contain arbitrary remote text. Retain only a typed failure.
    require(r.returncode == 0, 'local_command_failed:' + Path(argv[0]).name + ':' + str(r.returncode))
    return r.stdout


def choose_release(releases, prefix, package, target, pin=None):
    candidates = []
    for r in releases:
        tag = r['tag_name']
        if r.get('draft') or r.get('prerelease') or not tag.startswith(prefix):
            continue
        value = tag[len(prefix):]
        if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', value) or (pin and pin != value):
            continue
        filename = f'{package}-v{value}-{target}.tar.gz' if target else 'install.sh'
        if filename in {a['name'] for a in r.get('assets', [])}:
            candidates.append((semver(value), value, r))
    require(bool(candidates), 'published_package_not_found')
    _, version, release = max(candidates, key=lambda c: c[0])
    return version, release


def published(repo, prefix, package, target, pin=None):
    if pin:
        r = api(f'repos/{repo}/releases/tags/{prefix}{pin}')
        return choose_release([r], prefix, package, target, pin)
    # Bounded pagination; never substitute a library release or source build.
    releases = []
    for page in range(1, 6):
        batch = api(f'repos/{repo}/releases?per_page=100&page={page}')
        releases.extend(batch)
        if len(batch) < 100:
            break
    return choose_release(releases, prefix, package, target)


def download_release(repo, release, filename, index, work):
    assets = {a['name']: a for a in release['assets']}
    require(filename in assets and index in assets, 'required_release_asset_missing')
    sha = checksums(fetch(assets[index]['browser_download_url'], 65536).decode())[filename]
    p = work / filename
    p.write_bytes(fetch(assets[filename]['browser_download_url'], 100 * 1024 * 1024))
    require(p.stat().st_size == assets[filename]['size'] and digest(p) == sha, 'archive_checksum_mismatch')
    return p


def attest(gh, archive, repo, work, *, workflow=None, ref=None, source=None):
    sha = digest(archive)
    bundles = api(f'repos/{repo}/attestations/sha256:{sha}').get('attestations', [])
    require(bool(bundles), 'attestation_missing')
    bundle = work / (sha + '.jsonl')
    bundle.write_text(''.join(json.dumps(a['bundle']) + '\n' for a in bundles))
    args = [gh, 'attestation', 'verify', archive, '--bundle', bundle, '--repo', repo, '--format', 'json']
    if workflow:
        args += ['--signer-workflow', f'{repo}/.github/workflows/{workflow}']
    if ref:
        args += ['--source-ref', 'refs/tags/' + ref]
    if source:
        args += ['--source-digest', source]
    result = json.loads(execute(args, config=work / 'empty-gh-config'))
    require(bool(result), 'attestation_not_verified')
    return result


def freeze(package):
    for p in package.rglob('*'):
        p.chmod(0o500 if p.is_dir() or p.stat().st_mode & 0o111 else 0o400)
    package.chmod(0o500)


def activate(link, target):
    # Previous version trees are retained. Only the selected entrypoint changes.
    temp = link.with_name('.' + link.name + '-setup-' + str(os.getpid()))
    require(not temp.exists() and not temp.is_symlink(), 'activation_collision')
    temp.symlink_to(target)
    os.replace(temp, link)


def install_tree(candidate, destination):
    if destination.exists():
        require(destination.is_dir() and not destination.is_symlink(), 'unsafe_package_directory')
        require(all(not p.is_symlink() for p in destination.rglob('*')), 'package_links_forbidden')
        actual = {str(p.relative_to(destination)): digest(p) for p in destination.rglob('*') if p.is_file()}
        expected = {str(p.relative_to(candidate)): digest(p) for p in candidate.rglob('*') if p.is_file()}
        require(actual == expected, 'existing_package_integrity_mismatch')
        for name in expected:
            mode = (destination / name).stat().st_mode
            require(not mode & 0o222 and bool(mode & 0o111) == bool((candidate / name).stat().st_mode & 0o111), 'existing_package_mode_mismatch')
    else:
        shutil.move(candidate, destination)
        freeze(destination)
    return destination


def bootstrap_gh(base, work):
    seed = work / 'gh-seed.tar.gz'
    seed.write_bytes(fetch(f'https://github.com/cli/cli/releases/download/v{SEED_GH}/gh_{SEED_GH}_linux_amd64.tar.gz', 64 * 1024 * 1024))
    require(digest(seed) == SEED_SHA, 'gh_bootstrap_hash_mismatch')
    candidate = unpack(seed, work / 'gh-seed')
    seed_path = install_tree(candidate, base / 'tools' / ('gh-' + SEED_GH + '-' + SEED_SHA[:12])) / 'bin/gh'
    latest = api('repos/cli/cli/releases/latest')
    version = latest['tag_name'].removeprefix('v')
    require(semver(version) >= semver(SEED_GH), 'gh_release_downgrade')
    selected = seed_path
    if version != SEED_GH:
        filename = f'gh_{version}_linux_amd64.tar.gz'
        archive = download_release('cli/cli', latest, filename, f'gh_{version}_checksums.txt', work)
        proof = attest(seed_path, archive, 'cli/cli', work)
        candidate = unpack(archive, work / 'gh-latest')
        selected = install_tree(candidate, base / 'tools' / ('gh-' + version + '-' + digest(archive)[:12])) / 'bin/gh'
        (base / 'receipts' / ('gh-' + version + '.json')).write_text(json.dumps(proof))
    execute([selected, '--version'])
    activate(base / 'bin/gh', selected)
    emit('verification_tool_ready', gh_version=version, global_gh_required=False, github_login_required=False)
    return selected


def install_local_model(base, bindir, work):
    runtime_url = 'https://github.com/ggml-org/llama.cpp/releases/download/b11913/llama-b11913-bin-ubuntu-x64.tar.gz'
    runtime_sha = 'c670ad826f59478db859c9d4a0d931ab9eb653f22b7e0adebd70816c4092aab5'
    model_sha = 'ca59ca7f13d0e15a8cfa77bd17e65d24f6844b554a7b6c12e07a5f89ff76844e'
    revision = '9217f5db79a29953eb74d5343926648285ec7e67'
    model_url = 'https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/' + revision + '/qwen2.5-0.5b-instruct-q8_0.gguf'
    archive = work / 'llama-runtime.tar.gz'
    archive.write_bytes(fetch(runtime_url, 64 * 1024 * 1024))
    require(digest(archive) == runtime_sha, 'local_model_runtime_hash_mismatch')
    candidate = unpack(archive, work / 'llama-runtime', materialize_links=True)
    runtime = install_tree(candidate, base / ('llama-b11913-' + runtime_sha[:12]))
    model = base / ('qwen2.5-0.5b-instruct-q8_0-' + model_sha[:12] + '.gguf')
    if model.exists():
        require(not model.is_symlink() and digest(model) == model_sha, 'local_model_hash_mismatch')
    else:
        partial = work / 'model.gguf'
        with urllib.request.urlopen(model_url, timeout=60) as response, partial.open('wb') as f:
            size = 0
            while chunk := response.read(1024 * 1024):
                size += len(chunk)
                require(size <= 700 * 1024 * 1024, 'local_model_download_limit')
                f.write(chunk)
        require(digest(partial) == model_sha, 'local_model_hash_mismatch')
        shutil.move(partial, model)
        model.chmod(0o400)
    license_url = 'https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/raw/' + revision + '/LICENSE'
    (base / 'Qwen-LICENSE').write_bytes(fetch(license_url, 65536))
    wrapper_text = '#!/bin/sh\nexec env -i HOME="$HOME" PATH=/usr/bin:/bin LD_LIBRARY_PATH=' + shlex.quote(str(runtime)) + ' ' + shlex.quote(str(runtime / 'llama-completion')) + ' -m ' + shlex.quote(str(model)) + ' -t 2 -tb 2 -ngl 0 "$@"\n'
    wrapper = base / ('local-model-' + VERSION + '-' + hashlib.sha256(wrapper_text.encode()).hexdigest()[:12] + '.sh')
    if wrapper.exists():
        require(wrapper.read_text() == wrapper_text, 'local_model_wrapper_mismatch')
    else:
        wrapper.write_text(wrapper_text)
        wrapper.chmod(0o500)
    activate(bindir / 'zix-local-model', wrapper)
    receipt = {'runtime': 'llama.cpp b11913', 'runtime_url': runtime_url, 'runtime_sha256': runtime_sha, 'model': 'Qwen2.5-0.5B-Instruct q8_0', 'model_url': model_url, 'model_sha256': model_sha, 'model_revision': revision, 'license_url': license_url, 'selection': 'tested_pinned_pair_not_latest_model', 'secrets_inherited': False}
    (base / 'local-model.json').write_text(json.dumps(receipt, indent=2))
    emit('local_model_ready', command='zix-local-model', model='0.5B q8_0', runtime='llama.cpp b11913')


def entries(zix):
    return {e['namespace']: e for e in json.loads(execute([zix, 'extensions', 'list', '--format', 'json']))['entries']}


def system_check():
    require(sys.version_info >= (3, 12), 'Python_3.12_required')
    require(platform.system() == 'Linux' and platform.machine() == 'x86_64', 'supported_platform:Linux_x86_64')
    require(os.getuid() != 0, 'run_as_your_login_user_not_root')
    libc = platform.libc_ver()
    require(libc[0] == 'glibc' and tuple(map(int, libc[1].split('.')[:2])) >= (2, 39), 'glibc_2.39_required:use_Ubuntu_24.04_or_newer')
    try:
        ctypes.CDLL('libsecret-1.so.0')
    except OSError:
        raise ValueError('libsecret_missing:Ubuntu:sudo apt-get install libsecret-1-0') from None


def doctor(zix):
    report = {'installed_cli': zix.exists(), 'path_selects_managed_cli': shutil.which('zix') == str(zix), 'dbus_session_address_present': bool(os.environ.get('DBUS_SESSION_BUS_ADDRESS')), 'secret_service_owned': False}
    if shutil.which('busctl'):
        r = subprocess.run(['busctl', '--user', '--no-pager', '--no-legend', 'list'], capture_output=True, timeout=10)
        report['secret_service_owned'] = r.returncode == 0 and any(line.split()[0] == b'org.freedesktop.secrets' for line in r.stdout.splitlines() if line.split())
    if zix.exists():
        report['cli_version'] = execute([zix, 'version']).decode().strip()
        report['extensions'] = [{k: e[k] for k in ['namespace', 'version', 'enabled']} for e in entries(zix).values()]
    emit('local_status', **report, next='If custody is unavailable, configure a login-session Secret Service and unlock it; no credentials have been read.')
    return report


def install_package(namespace, zix, packages, gh, work, pin=None, allow_downgrade=False):
    name, package, libc, prefix, workflow = RECIPES[namespace]
    repo = 'zixcel/' + name
    target = 'x86_64-unknown-linux-' + libc
    version, release = published(repo, prefix, package, target, pin)
    old = entries(zix).get(namespace) if namespace != 'cli' else None
    if old and not allow_downgrade:
        require(semver(version) >= semver(old['version']), 'extension_downgrade_requires_allow_downgrade')
    if namespace == 'cli' and zix.exists() and not allow_downgrade:
        current = execute([zix, 'version']).decode().strip().removeprefix('zix ')
        require(semver(version) >= semver(current), 'cli_downgrade_requires_allow_downgrade')
    sub = work / namespace
    sub.mkdir()
    archive = download_release(repo, release, f'{package}-v{version}-{target}.tar.gz', 'SHA256SUMS.gnu' if namespace == 'cli' else 'SHA256SUMS', sub)
    candidate = unpack(archive, sub / 'unpack')
    verify_contents(candidate)
    record = json.loads((candidate / 'RELEASE.json').read_text())
    binary = 'zix' if namespace == 'cli' else 'zix-' + namespace
    for k, v in {'schema': 'zix-native-release/v1', 'package': package, 'version': version, 'repository': 'https://github.com/' + repo, 'target': target, 'binary': binary}.items():
        require(record.get(k) == v, 'release_binding_mismatch:' + k)
    require(bool(re.fullmatch('[a-f0-9]{40}', record['source'])), 'invalid_release_source')
    require(digest(candidate / binary) == record['binary_sha256'], 'release_binary_mismatch')
    proof = attest(gh, archive, repo, sub, workflow=workflow, ref=release['tag_name'], source=record['source'])
    if namespace == 'cli':
        require('credential-store' in record.get('features', []), 'credential_store_package_required')
        require(execute([candidate / binary, 'version']).decode().strip() == 'zix ' + version, 'binary_version_mismatch')
    else:
        manifest = json.loads((candidate / (namespace + '.extension.json')).read_text())
        require(manifest['namespace'] == namespace and manifest['package'] == package and manifest['version'] == version, 'manifest_identity_mismatch')
        for option, suffix, key in [('--manifest', 'extension', 'manifest_sha256'), ('--guidance', 'guidance', 'guidance_sha256')]:
            f = candidate / (namespace + '.' + suffix + '.json')
            require(digest(f) == record[key] and execute([candidate / binary, option]) == f.read_bytes(), 'embedded_definition_mismatch')
    destination = install_tree(candidate, packages / (namespace + '-' + version + '-' + digest(archive)[:12]))
    if namespace == 'cli':
        previous = str(zix.resolve()) if zix.exists() else None
        if zix.exists() and not zix.is_symlink():
            backup = packages / ('previous-cli-' + digest(zix))
            if not backup.exists():
                shutil.copy2(zix, backup)
        activate(zix, destination / binary)
    else:
        previous = old
        args = [zix, 'extensions', 'replace' if old else 'add', '--manifest', destination / (namespace + '.extension.json'), '--manifest-sha256', record['manifest_sha256'], '--executable', destination / binary, '--executable-sha256', record['binary_sha256']]
        if old:
            args += ['--expected-manifest-sha256', old['manifest_sha256'], '--expected-executable-sha256', old['executable_sha256']]
        execute(args)
    return {'namespace': namespace, 'version': version, 'source': record['source'], 'tag': release['tag_name'], 'archive_sha256': digest(archive), 'package_directory': str(destination), 'attestation_verified': True, 'attestation': proof, 'previous': previous}


def main():
    parser = argparse.ArgumentParser(prog='zix-setup', description='Install/update attested Zix packages and a private verification helper. No provider login or API writes.')
    parser.add_argument('action', nargs='?', default='install', choices=['install', 'update', 'doctor'])
    parser.add_argument('--with', dest='providers', action='append', default=[], help='Comma-separated provider namespaces; repeatable')
    parser.add_argument('--pin', action='append', default=[], metavar='NAMESPACE=VERSION')
    parser.add_argument('--local-model', action='store_true', help='Install the pinned, tested optional 0.5B sample runtime/model (about 0.8 GB)')
    parser.add_argument('--allow-downgrade', action='store_true')
    parser.add_argument('--no-self-update', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args(sys.argv[2:])
    system_check()
    os.umask(0o077)
    home = Path.home()
    base = home / '.local/share/zix/setup'
    packages = home / '.local/share/zix/packages'
    bindir = home / '.local/bin'
    for path in [base, packages, base / 'tools', base / 'bin', base / 'receipts']:
        private_dir(path)
    private_dir(bindir, private=False)
    zix = bindir / 'zix'
    if args.action == 'doctor':
        doctor(zix)
        return
    providers = [n for group in args.providers for n in group.split(',')]
    pins = {}
    for pin in args.pin:
        name, value = pin.split('=', 1)
        require(name in RECIPES, 'unknown_pin_namespace')
        semver(value)
        require(name not in pins, 'duplicate_pin')
        pins[name] = value
    require(all(n in RECIPES and n != 'cli' for n in providers), 'unknown_provider')
    lock_path = base / '.lock'
    lock_fd = os.open(lock_path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    with os.fdopen(lock_fd, 'w') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise ValueError('setup_already_running') from None
        with tempfile.TemporaryDirectory(prefix='.download-', dir=base) as tmp:
            work = Path(tmp)
            gh = bootstrap_gh(base, work)
            if args.action == 'update' and not args.no_self_update:
                # Hand off after releasing our lock, using a separately retained stage.
                version, release = published('zixcel/cli', 'installer/v', '', '')
                require(semver(version) >= semver(VERSION), 'installer_release_downgrade')
                if semver(version) > semver(VERSION):
                    candidate = download_release('zixcel/cli', release, 'install.sh', 'SHA256SUMS.installer', work)
                    attest(gh, candidate, 'zixcel/cli', work, workflow='release-installer.yml', ref=release['tag_name'])
                    fcntl.flock(lock, fcntl.LOCK_UN)
                    subprocess.run(['/bin/bash', str(candidate), *sys.argv[2:], '--no-self-update'], check=True)
                    return
            if args.action == 'update' and zix.exists():
                providers += [n for n in entries(zix) if n in RECIPES and n != 'cli']
            require(set(pins) <= {'cli', *providers}, 'pin_requires_selected_provider')
            script = Path(sys.argv[1]).resolve()
            installer = base / ('install-' + VERSION + '-' + digest(script)[:12] + '.sh')
            if not installer.exists():
                shutil.copyfile(script, installer)
                installer.chmod(0o500)
            require(digest(installer) == digest(script), 'installed_setup_hash_mismatch')
            activate(bindir / 'zix-setup', installer)
            stamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
            receipt_path = base / 'receipts' / (stamp + '.json')
            receipt = {'schema': 'zix-setup-receipt/v1', 'installer_version': VERSION, 'state': 'in_progress', 'packages': []}
            try:
                for n in dict.fromkeys(['cli', *providers]):
                    emit('installing', namespace=n, selection=pins.get(n, 'latest_stable_native'))
                    row = install_package(n, zix, packages, gh, work, pins.get(n), args.allow_downgrade)
                    receipt['packages'].append(row)
                    receipt_path.write_text(json.dumps(receipt, indent=2))
                    emit('installed', namespace=n, version=row['version'], attestation_verified=True)
                if args.local_model or (args.action == 'update' and (base / 'local-model.json').exists()):
                    install_local_model(base, bindir, work)
                    receipt['local_model'] = 'tested_pinned_pair'
                receipt['state'] = 'complete'
            except Exception:
                receipt['state'] = 'stopped_partial'
                raise
            finally:
                receipt_path.write_text(json.dumps(receipt, indent=2))
            doctor(zix)
            emit('complete', receipt=str(receipt_path), path_setup='export PATH="$HOME/.local/bin:$PATH"', next='zix-setup doctor; then inspect capabilities and enroll provider credentials separately')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.SubprocessError, tarfile.TarError) as error:
        message = str(error) if isinstance(error, ValueError) else type(error).__name__
        emit('stopped', reason=message, retry='no_automatic_retry', previous_packages='retained', provider_write_attempted=False)
        sys.exit(2)
PY
