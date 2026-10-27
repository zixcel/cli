"""Offline adversarial tests; never execute downloaded code or contact services."""
import hashlib
import io
import json
from pathlib import Path
import tarfile
import tempfile
import types
import unittest
from unittest.mock import patch

module = types.ModuleType('setup_under_test')
source = Path(__file__).with_name('install.sh').read_text().split("<<'PY'\n", 1)[1].rsplit('\nPY', 1)[0]
exec(compile(source, 'install.sh:python', 'exec'), module.__dict__)


class SetupTests(unittest.TestCase):
    def release(self, tag, filename, **kw):
        return {'tag_name': tag, 'assets': [{'name': filename}], **kw}

    def test_latest_native_does_not_select_library_prerelease_or_incomplete(self):
        rs = [self.release('v9.0.0', 'library.zip'), self.release('google/v0.7.0', 'pkg-v0.7.0-linux.tar.gz'), self.release('google/v0.8.0', 'pkg-v0.8.0-linux.tar.gz', prerelease=True), self.release('google/v0.9.0', 'source.zip')]
        self.assertEqual(module.choose_release(rs, 'google/v', 'pkg', 'linux')[0], '0.7.0')
        with self.assertRaisesRegex(ValueError, 'published_package_not_found'):
            module.choose_release(rs, 'google/v', 'pkg', 'linux', '0.9.0')

    def test_numeric_version_sort(self):
        rs = [self.release('v0.9.0', 'pkg-v0.9.0-linux.tar.gz'), self.release('v0.10.0', 'pkg-v0.10.0-linux.tar.gz')]
        self.assertEqual(module.choose_release(rs, 'v', 'pkg', 'linux')[0], '0.10.0')

    def test_checksum_paths_and_duplicates_reject(self):
        sha = 'a' * 64
        for text in [f'{sha}  ../escape', f'{sha}  /escape', f'{sha}  a\n{sha}  a', 'garbage']:
            with self.subTest(text=text), self.assertRaises(ValueError):
                module.checksums(text)
        self.assertEqual(module.checksums(f'{sha} *a'), {'a': sha})

    def test_archive_traversal_links_and_duplicates_reject_before_extraction(self):
        for names, link in [(['../escape'], False), (['/escape'], False), (['p/a'], True), (['p/a', 'p/a'], False)]:
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                with tarfile.open(root / 'archive.tgz', 'w:gz') as t:
                    for name in names:
                        m = tarfile.TarInfo(name)
                        if link:
                            m.type = tarfile.SYMTYPE
                            m.linkname = '/etc/passwd'
                        else:
                            m.size = 1
                        t.addfile(m, None if link else io.BytesIO(b'x'))
                with self.assertRaises(ValueError):
                    module.unpack(root / 'archive.tgz', root / 'out')
                self.assertEqual(list((root / 'out').iterdir()), [])

    def test_runtime_links_materialize_only_internal_regular_files(self):
        for target, ok in [('library.so.0', True), ('/etc/passwd', False), ('../escape', False), ('link.so', False)]:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as tmp:
                p = Path(tmp)
                with tarfile.open(p / 'runtime.tgz', 'w:gz') as t:
                    m = tarfile.TarInfo('runtime/library.so.0'); m.size = 3
                    t.addfile(m, io.BytesIO(b'lib'))
                    m = tarfile.TarInfo('runtime/link.so'); m.type = tarfile.SYMTYPE; m.linkname = target
                    t.addfile(m)
                if ok:
                    root = module.unpack(p / 'runtime.tgz', p / 'out', materialize_links=True)
                    self.assertFalse((root / 'link.so').is_symlink())
                    self.assertEqual((root / 'link.so').read_bytes(), b'lib')
                else:
                    with self.assertRaises(ValueError):
                        module.unpack(p / 'runtime.tgz', p / 'out', materialize_links=True)

    def test_package_extra_missing_or_corrupt_file_rejects(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp)
            (p / 'a').write_bytes(b'valid')
            (p / 'CONTENTS.sha256').write_text(hashlib.sha256(b'valid').hexdigest() + '  a\n')
            module.verify_contents(p)
            (p / 'a').write_bytes(b'corrupt')
            with self.assertRaisesRegex(ValueError, 'contents_mismatch'):
                module.verify_contents(p)
            (p / 'extra').touch()
            with self.assertRaisesRegex(ValueError, 'unexpected_package_files'):
                module.verify_contents(p)

    def test_attestation_uses_public_bundle_without_saved_auth(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp)
            archive = p / 'archive'
            archive.write_bytes(b'archive')
            with patch.object(module, 'api', return_value={'attestations': [{'bundle': {'test': True}}]}), patch.object(module, 'execute', return_value=b'[{"verified":true}]') as run:
                module.attest('/private/gh', archive, 'owner/repo', p, workflow='release.yml', ref='v1.0.0', source='a' * 40)
            argv = run.call_args.args[0]
            self.assertIn('--bundle', argv)
            self.assertIn('--source-digest', argv)
            self.assertEqual(argv[argv.index('--signer-workflow') + 1], 'owner/repo/.github/workflows/release.yml')
            with patch.object(module, 'api', return_value={'attestations': []}), self.assertRaisesRegex(ValueError, 'attestation_missing'):
                module.attest('/private/gh', archive, 'owner/repo', p)

    def test_child_environment_excludes_tokens_and_uses_empty_gh_config(self):
        result = types.SimpleNamespace(returncode=0, stdout=b'ok')
        with patch.dict(module.os.environ, {'GH_TOKEN': 'not-for-child', 'ZIX_GITHUB_ACCESS_TOKEN': 'not-for-child'}), patch.object(module.subprocess, 'run', return_value=result) as run:
            module.execute(['/bin/example'], config=Path('/private/empty'))
        env = run.call_args.kwargs['env']
        self.assertNotIn('GH_TOKEN', env)
        self.assertNotIn('ZIX_GITHUB_ACCESS_TOKEN', env)
        self.assertEqual(env['GH_CONFIG_DIR'], '/private/empty')

    def test_existing_package_mutation_and_symlinks_reject(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            a, b = root / 'candidate', root / 'installed'
            a.mkdir(); b.mkdir()
            (a / 'binary').write_bytes(b'official')
            (b / 'binary').write_bytes(b'modified')
            with self.assertRaisesRegex(ValueError, 'integrity_mismatch'):
                module.install_tree(a, b)
            (b / 'binary').unlink(); (b / 'binary').symlink_to(a / 'binary')
            with self.assertRaisesRegex(ValueError, 'links_forbidden'):
                module.install_tree(a, b)

    def test_failed_registration_keeps_current_registration_and_no_activation(self):
        # A registration is replaced using its observed hashes, never removed first.
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp); candidate = p / 'candidate'; candidate.mkdir(); packages = p / 'packages'; packages.mkdir()
            record = {'schema': 'zix-native-release/v1', 'package': 'zixcel-github-cli', 'version': '0.2.0', 'repository': 'https://github.com/zixcel/zixcel-github', 'target': 'x86_64-unknown-linux-musl', 'binary': 'zix-github', 'source': 'a' * 40}
            manifest = {'namespace': 'github', 'package': record['package'], 'version': '0.2.0'}
            files = {'zix-github': b'binary', 'github.extension.json': json.dumps(manifest).encode(), 'github.guidance.json': b'{}'}
            for key, name in [('binary_sha256', 'zix-github'), ('manifest_sha256', 'github.extension.json'), ('guidance_sha256', 'github.guidance.json')]:
                record[key] = hashlib.sha256(files[name]).hexdigest()
            files['RELEASE.json'] = json.dumps(record).encode()
            for name, value in files.items(): (candidate / name).write_bytes(value)
            (candidate / 'CONTENTS.sha256').write_text(''.join(hashlib.sha256(value).hexdigest() + '  ' + name + '\n' for name, value in files.items()))
            archive = p / 'archive'; archive.write_bytes(b'archive')
            calls = []
            def run(argv, **kw):
                calls.append(argv)
                if 'replace' in argv: raise ValueError('stale_registration')
                return files['github.extension.json'] if '--manifest' in argv else files['github.guidance.json']
            old = {'version': '0.1.0', 'manifest_sha256': 'old-manifest', 'executable_sha256': 'old-executable', 'enabled': False}
            with patch.object(module, 'published', return_value=('0.2.0', {'tag_name': 'native/github/v0.2.0'})), patch.object(module, 'entries', return_value={'github': old}), patch.object(module, 'download_release', return_value=archive), patch.object(module, 'unpack', return_value=candidate), patch.object(module, 'attest', return_value=[{}]), patch.object(module, 'install_tree', return_value=candidate), patch.object(module, 'execute', side_effect=run):
                with self.assertRaisesRegex(ValueError, 'stale_registration'):
                    module.install_package('github', p / 'zix', packages, p / 'gh', p)
            argv = calls[-1]
            self.assertIn('replace', argv); self.assertNotIn('remove', argv)
            self.assertEqual(argv[argv.index('--expected-manifest-sha256') + 1], 'old-manifest')
            self.assertFalse(old['enabled'])


if __name__ == '__main__':
    unittest.main()
