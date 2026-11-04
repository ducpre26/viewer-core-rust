"""Read-only checks against the study in STORE.md and Rust DICOM passthrough.

Stores real metadata/media only under ignored target/. No remote writes.
Server-rendered JPEG is tested separately from the local Java JPEG renderer.
"""
import argparse
import base64
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.parse
import urllib.request
import zipfile

import config_fixture
from smoke_wado import configured
from smoke_app import stop
import smoke_jni

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'target/step-12-logs/live-store'
sys.path.insert(0, str(ROOT / 'target/step-12-tools'))

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None

HTTP = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())

def request(url, auth=None, accept=None, body=None, method=None):
    headers = {}
    if auth:
        headers['Authorization'] = auth
    if accept:
        headers['Accept'] = accept
    if body is not None:
        body = json.dumps(body).encode()
        headers['Content-Type'] = 'application/json'
    req = urllib.request.Request(url, data=body, headers=headers, method=method)
    with HTTP.open(req, timeout=30) as response:
        data = response.read(128 * 1024 * 1024 + 1)
        if len(data) > 128 * 1024 * 1024:
            raise RuntimeError('Sample exceeded the 128 MiB test limit')
        return response.status, response.headers, data

def tag(dataset, name):
    return str(dataset[name]['Value'][0])

def main():
    from PIL import Image
    import pydicom
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--samples', type=int, default=3)
    parser.add_argument('--java-home', type=Path, help='Also verify study/series ZIP with real Java metadata parsing; no native codec')
    args = parser.parse_args()
    if not 1 <= args.samples <= 10:
        raise ValueError('Use 1..10 read-only samples')
    lines = (ROOT / 'docs/STORE.md').read_text(encoding='utf-8').splitlines()
    store = next(line[4:].strip() for line in lines if line.startswith('URL '))
    study = next(line[6:].strip() for line in lines if line.startswith('SIUID '))
    user, password = (value.strip() for value in next(line[5:].strip() for line in lines if line.startswith('auth ')).split('/', 1))
    auth = 'Basic ' + base64.b64encode(f'{user}:{password}'.encode()).decode()
    OUT.mkdir(parents=True, exist_ok=True)
    path = f'{store.rstrip("/")}/studies/{urllib.parse.quote(study, safe="")}/series'
    status, _, payload = request(path, auth, 'application/dicom+json')
    (OUT / 'series.json').write_bytes(payload)
    series = json.loads(payload)
    assert status == 200 and isinstance(series, list) and series
    series_uid = tag(series[0], '0020000E')
    path = f'{store.rstrip("/")}/studies/{study}/series/{series_uid}/instances'
    status, _, payload = request(path, auth, 'application/dicom+json')
    (OUT / 'instances.json').write_bytes(payload)
    instances = json.loads(payload)
    assert status == 200 and isinstance(instances, list) and instances
    samples = instances[:args.samples]

    # Independent assertions on the live metadata endpoint through the local CLI.
    cli = shutil.which('postman')
    if not cli:
        raise RuntimeError('Postman CLI required for the live API assertions')
    with tempfile.TemporaryDirectory(prefix='live-postman-', dir=ROOT / 'target') as temporary:
        environment = Path(temporary) / 'environment.json'
        environment.write_text(json.dumps({'name': 'local-only live Orthanc', 'values': [
            {'key': 'probeUrl', 'value': path, 'enabled': True},
            {'key': 'authorization', 'value': auth, 'type': 'secret', 'enabled': True},
        ]}), encoding='utf-8')
        script = "pm.test('HTTP 200',()=>pm.response.to.have.status(200));pm.test('JSON array',()=>pm.expect(pm.response.json()).to.be.an('array'));pm.test('SOP UID present',()=>pm.expect(pm.response.json()[0]['00080018'].Value[0]).to.be.a('string'));"
        result = subprocess.run([cli, 'request', '{{probeUrl}}', '-e', str(environment),
            '-H', 'Authorization:{{authorization}}', '-H', 'Accept:application/dicom+json',
            '--script-post-request', script, '--timeout', '30000', '--no-report-events'], cwd=ROOT,
            capture_output=True, text=True, encoding='utf-8', errors='replace', timeout=120)
        (OUT / 'postman.log').write_text((result.stdout + result.stderr).replace(auth, '<redacted>'), encoding='utf-8')
        assert result.returncode == 0, 'Postman assertions failed; see local postman.log'

    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    process = None
    session = None
    records = []
    archives = []
    with tempfile.TemporaryDirectory(prefix='live-download-', dir=ROOT / 'target') as temporary:
        working = Path(temporary).resolve()
        system_temp = working / 'system-temp'
        system_temp.mkdir()
        files = system_temp / 'pacs-viewer-core-downloads'
        config = config_fixture.merge(configured(), {
            'web': {'binding': '127.0.0.1', 'port': port, 'middlewares': {'logger': {'enable': False}}},
            'auth': {'client_security_disabled': True}, 'jni': {'enabled': False},
            'logger': {'level': 'warn'},
            'video': {'fps': 1},
        })
        config.pop('download', None)
        config_fixture.write(working / 'config/app.toml', config)
        env = os.environ.copy()
        env.update(SUMMER_ENV='dev', RUST_LOG='warn', TEMP=str(system_temp), TMP=str(system_temp), TMPDIR=str(system_temp))
        options = {}
        if os.name == 'nt':
            import ctypes
            ctypes.windll.kernel32.SetConsoleCtrlHandler(None, False)
            startup = subprocess.STARTUPINFO()
            startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
            startup.wShowWindow = subprocess.SW_HIDE
            options.update(creationflags=subprocess.CREATE_NEW_CONSOLE, startupinfo=startup)
        base = f'http://127.0.0.1:{port}/ws/rest'
        binary = ROOT / 'target/debug' / ('viewer-app.exe' if os.name == 'nt' else 'viewer-app')
        with (OUT / 'app.log').open('wb') as log:
            try:
                process = subprocess.Popen([str(binary)], cwd=working, env=env, stdout=log, stderr=log, **options)
                deadline = time.monotonic() + 30
                while True:
                    assert process.poll() is None, 'Application startup failed; see app.log'
                    try:
                        if request(base + '/ready')[0] == 200:
                            break
                    except (OSError, urllib.error.URLError):
                        pass
                    if time.monotonic() > deadline:
                        raise RuntimeError('Application startup timeout')
                    time.sleep(0.1)
                status, _, payload = request(base + '/client/link', body={
                    'storeUrl': store, 'storeAuth': auth, 'studies': [study], 'expireIn': 5,
                })
                assert status == 200
                session = json.loads(payload)['session']['id']
                for index, instance in enumerate(samples, 1):
                    sop = tag(instance, '00080018')
                    prefix = f'sample-{index:02d}'
                    metadata_path = f'{store.rstrip("/")}/studies/{study}/series/{series_uid}/instances/{sop}/metadata'
                    _, _, metadata = request(metadata_path, auth, 'application/dicom+json')
                    (OUT / f'{prefix}-metadata.json').write_bytes(metadata)
                    endpoint = store.rsplit('/', 1)[0].rstrip('/') + '/wado'
                    query = {'studyUID': study, 'seriesUID': series_uid, 'objectUID': sop, 'requestType': 'WADO'}
                    source_url = endpoint + '?' + urllib.parse.urlencode(dict(query, contentType='application/dicom'))
                    source_status, source_headers, source = request(source_url, auth)
                    assert source_status == 200 and source_headers.get_content_type() == 'application/dicom'
                    dataset = pydicom.dcmread(io.BytesIO(source))
                    assert str(dataset.SOPInstanceUID) == sop
                    assert str(dataset.StudyInstanceUID) == study and str(dataset.SeriesInstanceUID) == series_uid
                    assert dataset.Rows > 0 and dataset.Columns > 0 and 'PixelData' in dataset
                    pixel_decoded = False
                    if str(dataset.file_meta.TransferSyntaxUID) == '1.2.840.10008.1.2.4.50':
                        from pydicom.encaps import generate_frames
                        frame = next(generate_frames(dataset.PixelData, number_of_frames=int(getattr(dataset, 'NumberOfFrames', 1))))
                        with Image.open(io.BytesIO(frame)) as image:
                            image.load()
                            assert image.size == (dataset.Columns, dataset.Rows)
                        pixel_decoded = True
                    (OUT / f'{prefix}-source.dcm').write_bytes(source)
                    download = f'{base}/v1/session/{session}/download/studies/{study}/series/{series_uid}/instances/{sop}?downloadType=dicom'
                    rust_status, rust_headers, received = request(download)
                    assert rust_status == 200 and rust_headers.get_content_type() == 'application/dicom'
                    assert received == source, 'Rust DICOM bytes differ from upstream'
                    assert rust_headers['Content-Disposition'] == f'attachment;filename={sop}.dcm'
                    (OUT / f'{prefix}-download.dcm').write_bytes(received)
                    jpeg_url = endpoint + '?' + urllib.parse.urlencode(dict(query, contentType='image/jpeg'))
                    jpeg_status, jpeg_headers, jpeg = request(jpeg_url, auth, 'image/jpeg')
                    wado_uri_type = jpeg_headers.get_content_type()
                    with Image.open(io.BytesIO(jpeg)) as candidate:
                        candidate.load()
                        wado_uri_format = candidate.format
                    (OUT / f'{prefix}-jpeg-response.json').write_text(json.dumps({
                        'status': jpeg_status, 'contentType': jpeg_headers.get('Content-Type'),
                        'bytes': len(jpeg), 'signatureHex': jpeg[:16].hex(),
                    }), encoding='utf-8')
                    if wado_uri_type != 'image/jpeg':
                        (OUT / f'{prefix}-wado-uri-image.bin').write_bytes(jpeg)
                    # Rust intentionally uses rendered-frame JPEG, independent of WADO-URI MIME bugs.
                    jpeg_url = metadata_path.removesuffix('/metadata') + '/frames/1/rendered?quality=90'
                    jpeg_status, jpeg_headers, jpeg = request(jpeg_url, auth, 'image/jpeg')
                    jpeg_endpoint = 'wado-rs-frame-rendered'
                    assert jpeg_status == 200 and jpeg_headers.get_content_type() == 'image/jpeg'
                    with Image.open(io.BytesIO(jpeg)) as image:
                        image.load()
                        assert image.format == 'JPEG' and image.size == (dataset.Columns, dataset.Rows)
                        mode = image.mode
                    (OUT / f'{prefix}-store.jpeg').write_bytes(jpeg)
                    jpeg_download = download.removesuffix('downloadType=dicom') + 'downloadType=jpeg'
                    rust_jpeg_status, rust_jpeg_headers, rust_jpeg = request(jpeg_download)
                    assert rust_jpeg_status == 200 and rust_jpeg_headers.get_content_type() == 'application/jpeg'
                    assert rust_jpeg_headers['Content-Disposition'] == f'attachment;filename={sop}.jpeg'
                    assert rust_jpeg == jpeg, 'Rust rendered JPEG differs from the store JPEG'
                    with Image.open(io.BytesIO(rust_jpeg)) as image:
                        image.load()
                        assert image.format == 'JPEG' and image.size == (dataset.Columns, dataset.Rows)
                    (OUT / f'{prefix}-download.jpeg').write_bytes(rust_jpeg)
                    assert not any(files.iterdir()), 'Rust job files remain after body completion'
                    records.append({'sample': index, 'dicomBytes': len(source), 'jpegBytes': len(jpeg),
                        'rows': int(dataset.Rows), 'columns': int(dataset.Columns),
                        'frames': int(getattr(dataset, 'NumberOfFrames', 1)),
                        'transferSyntax': str(dataset.file_meta.TransferSyntaxUID), 'jpegMode': mode,
                        'jpegEndpoint': jpeg_endpoint, 'wadoUriContentType': wado_uri_type,
                        'wadoUriActualFormat': wado_uri_format,
                        'sha256': hashlib.sha256(source).hexdigest(), 'rustByteExact': True, 'rustJpegByteExact': True})
                    records[-1]['dicomPixelDecodedIndependently'] = pixel_decoded
                    print(f'PASS live sample {index}: DICOM and JPEG byte-exact without JNI/Orthanc native', flush=True)
                if args.java_home and len(samples) == len(instances) and len(series) == 1:
                    request(base + '/v1/session/' + session, method='DELETE')
                    session = None
                    stop(process)
                    process = None
                    env = smoke_jni.environment(args.java_home)
                    env.update(SUMMER_ENV='dev', RUST_LOG='warn', TEMP=str(system_temp), TMP=str(system_temp), TMPDIR=str(system_temp))
                    config['jni'] = smoke_jni.jni_settings(args.java_home)
                    config_fixture.write(working / 'config/app.toml', config)
                    process = subprocess.Popen([str(binary)], cwd=working, env=env, stdout=log, stderr=log, **options)
                    deadline = time.monotonic() + 45
                    while True:
                        assert process.poll() is None, 'JNI metadata application startup failed'
                        try:
                            if request(base + '/ready')[0] == 200:
                                break
                        except (OSError, urllib.error.URLError):
                            pass
                        if time.monotonic() > deadline:
                            raise RuntimeError('JNI metadata application startup timeout')
                        time.sleep(0.1)
                    _, _, payload = request(base + '/client/link', body={
                        'storeUrl': store, 'storeAuth': auth, 'studies': [study], 'expireIn': 5,
                    })
                    session = json.loads(payload)['session']['id']
                    expected = {}
                    expected_jpeg = {}
                    for index in range(1, len(records)+1):
                        source = (OUT / f'sample-{index:02d}-source.dcm').read_bytes()
                        sop = str(pydicom.dcmread(io.BytesIO(source)).SOPInstanceUID)
                        expected[sop] = source
                        expected_jpeg[sop] = (OUT / f'sample-{index:02d}-store.jpeg').read_bytes()
                    for scope, suffix in [('series', '/series/' + series_uid), ('study', '')]:
                        for kind in ['dicom', 'jpeg']:
                            path = f'{base}/v1/session/{session}/download/studies/{study}{suffix}?downloadType={kind}'
                            status, headers, data = request(path)
                            assert status == 200 and headers.get_content_type() == 'application/zip'
                            with zipfile.ZipFile(io.BytesIO(data)) as archive:
                                actual = {}
                                for entry in archive.infolist():
                                    if not entry.is_dir():
                                        body = archive.read(entry)
                                        if kind == 'dicom':
                                            key = str(pydicom.dcmread(io.BytesIO(body)).SOPInstanceUID)
                                        else:
                                            key = entry.filename.rsplit('/', 1)[-1].removesuffix('.jpeg')
                                            with Image.open(io.BytesIO(body)) as image:
                                                image.load()
                                                assert image.format == 'JPEG'
                                        actual[key] = body
                                assert len(archive.infolist()) == len(expected) and actual == (expected if kind=='dicom' else expected_jpeg)
                            (OUT / f'{scope}-{kind}-download.zip').write_bytes(data)
                            assert not any(files.iterdir()), 'Archive job files remain after response'
                            archives.append({'scope': scope, 'type': kind, 'entries': len(actual), 'byteExact': True, 'jniForMetadataOnly': True})
                            print(f'PASS live {scope} ZIP {kind}: {len(actual)} store objects, no Orthanc native', flush=True)
            finally:
                if process is not None and process.poll() is None:
                    try:
                        if session:
                            request(base + '/v1/session/' + session, method='DELETE')
                    finally:
                        try:
                            stop(process)
                        except Exception:
                            if process.poll() is None:
                                process.kill()
                                process.wait(timeout=5)
                            raise
    result = {'source': 'docs/STORE.md', 'scope': 'specified study, first series, sampled instances',
        'seriesCount': len(series), 'instancesInFirstSeries': len(instances), 'postmanAssertions': 3,
        'jniEnabled': False, 'rustRemoteJpegVerified': True, 'localOrthancDependency': False,
        'wadoUriJpegHeaderConformant': all(r['wadoUriContentType'] == 'image/jpeg' for r in records),
        'samples': records, 'archives': archives}
    (OUT / 'result.json').write_text(json.dumps(result, indent=2)+'\n', encoding='utf-8')
    print(f'PASS {len(records)} real DICOM/JPEG samples via Rust; no Orthanc DLL required', flush=True)

if __name__ == '__main__':
    main()
