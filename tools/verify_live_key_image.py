"""Step 13 QA STOW/readback. Synthetic identities only; STORE.md is never logged.

Creates an isolated PostgreSQL database and synthetic QA images. By default
uses new studies; --reference-study appends a new series to the noted study.
Never deletes remote data.
"""
import base64
import io
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

import config_fixture
import smoke_jni
from smoke_app import stop
from smoke_wado import configured
from test_download_failures import process_options

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'target/step-13-logs/live-qa'
HTTP = urllib.request.build_opener(urllib.request.ProxyHandler({}))
sys.path.insert(0, str(ROOT / 'target/step-12-tools'))


def request(url, auth=None, body=None, media=None, accept=None, method=None):
    headers = {}
    if auth:
        headers['Authorization'] = auth
    if media:
        headers['Content-Type'] = media
    if accept:
        headers['Accept'] = accept
    req = urllib.request.Request(url, data=body, headers=headers, method=method)
    try:
        response = HTTP.open(req, timeout=15)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        return response.status, dict(response.headers), response.read()


def main():
    global OUT
    import argparse
    import pydicom
    from PIL import Image
    from pydicom.encaps import generate_frames
    from email.parser import BytesParser
    from email.policy import default
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java-home', required=True, type=Path)
    parser.add_argument('--store-file', type=Path, default=ROOT / 'docs/STORE.md')
    parser.add_argument('--output', type=Path, default=OUT)
    parser.add_argument('--reference-study', action='store_true',
                        help='Append synthetic key images to the study noted in the store file')
    parser.add_argument('--allow-unlisted-reference', action='store_true',
                        help='Use the explicitly confirmed study UID even if QIDO lists no source series')
    args = parser.parse_args()
    OUT = args.output.resolve()
    OUT.mkdir(parents=True, exist_ok=True)
    lines = args.store_file.read_text(encoding='utf-8').splitlines()
    urls = [x.strip().split() for x in lines if x.startswith('URL ')]
    store = next((parts[2] for parts in urls if len(parts) == 3 and parts[1].lower() == 'wadors'),
                 next((parts[1] for parts in urls if len(parts) == 2), '')).rstrip('/')
    assert store, 'Missing WADO-RS URL'
    stow_url = next((parts[2] for parts in urls if len(parts) == 3 and parts[1].lower() == 'stowrs'), store + '/studies').rstrip('/')
    assert stow_url == store + '/studies', 'Explicit STOW URL differs from the production session storeUrl/studies contract'
    reference = next(x[6:].strip() for x in lines if x.startswith('SIUID '))
    supplied_auth = next(x[5:].strip() for x in lines if x.startswith('auth '))
    if supplied_auth.lower().startswith(('basic ', 'bearer ')):
        auth = supplied_auth
    else:
        user, password = [x.strip() for x in supplied_auth.split('/', 1)]
        auth = 'Basic ' + base64.b64encode(f'{user}:{password}'.encode()).decode()
    config = configured()
    db = config['database']
    address = urllib.parse.urlsplit(db['url'].removeprefix('jdbc:'))
    database = 'viewer_step13_qa_' + uuid.uuid4().hex[:12]
    env = smoke_jni.environment(args.java_home)
    pg_env = os.environ.copy()
    pg_env['PGPASSWORD'] = db['password']
    psql = shutil.which('psql') or 'C:/Program Files/PostgreSQL/18/bin/psql.exe'

    def sql(query, name=database):
        result = subprocess.run([psql, '-X', '-h', address.hostname, '-p', str(address.port or 5432),
                                 '-U', db['username'], '-d', name, '-At', '-v', 'ON_ERROR_STOP=1', '-c', query],
                                env=pg_env, capture_output=True, timeout=20)
        if result.returncode:
            raise RuntimeError('PostgreSQL QA operation failed (credentials suppressed)')
        return result.stdout.decode().strip()

    result = {'target': 'QA from ' + args.store_file.name, 'referenceStudyReadOnly': not args.reference_study,
              'stowUrl': stow_url, 'isolatedDatabase': database, 'cases': [], 'passed': False}
    (OUT / 'result.json').write_text(json.dumps(result, indent=2))
    status, headers, data = request(f'{store}/studies/{reference}/series', auth, accept='application/dicom+json')
    content_type = next((value for key, value in headers.items() if key.lower() == 'content-type'), '')
    result['referenceQidoStatus'] = status
    result['referenceQidoContentType'] = content_type
    try:
        reference_series = json.loads(data)
    except (ValueError, UnicodeDecodeError):
        reference_series = None
    if status != 200 or not isinstance(reference_series, list):
        result['failureStage'] = 'reference-QIDO-preflight'
        result['failureReason'] = 'Expected DICOM JSON array; received HTTP ' + str(status) + ' / ' + content_type
        result['isolatedDatabaseCreated'] = False
        result['remoteWritesAttempted'] = False
        (OUT / 'result.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
        raise RuntimeError(result['failureReason'])
    result['referenceQidoStatus'] = status
    reference_attributes = {}
    shared_series = '2.25.' + str(uuid.uuid4().int)
    original_series = {row['0020000E']['Value'][0]: row.get('00201209', {}).get('Value', [None])[0]
                       for row in reference_series if row.get('0020000E', {}).get('Value')}
    if args.reference_study:
        if not original_series and not args.allow_unlisted_reference:
            result.update(referenceStudy=reference, failureStage='reference-study-lookup',
                          failureReason='No source series found for the noted study; original patient/study attributes unavailable',
                          isolatedDatabaseCreated=False, remoteWritesAttempted=False)
            (OUT / 'result.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
            raise RuntimeError(result['failureReason'])
        assert shared_series not in original_series, 'New series UID unexpectedly already exists'
        if original_series:
            first_series = next(iter(original_series))
            status, _, data = request(f'{store}/studies/{reference}/series/{first_series}/instances',
                                      auth, accept='application/dicom+json')
            instances = json.loads(data)
            assert status == 200 and instances, 'Reference study has no readable source instance'
            first_sop = instances[0]['00080018']['Value'][0]
            status, _, data = request(f'{store}/studies/{reference}/series/{first_series}/instances/{first_sop}/metadata',
                                      auth, accept='application/dicom+json')
            source_metadata = json.loads(data)
            assert status == 200 and source_metadata, 'Reference patient/study metadata unavailable'
            reference_attributes = {tag: value for tag, value in source_metadata[0].items()
                                    if tag.startswith('0010') or tag in {
                                        '00080020', '00080030', '00080050', '00080090',
                                        '00081030', '00200010', '00080080', '00080201'}}
        result.update(referenceStudy=reference, appendedSeries=shared_series,
                      originalSeries=original_series, patientStudyAttributesCopied=bool(reference_attributes),
                      referenceConfirmedByUser=args.allow_unlisted_reference)
    sql(f'CREATE DATABASE {database}', address.path.lstrip('/'))
    process = None
    working = OUT / database
    working.mkdir()
    files = working / 'files'
    fixture = json.loads((ROOT / 'docs/baseline/step-13-java.json').read_text())
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    base = f'http://127.0.0.1:{port}/ws/rest'
    config = config_fixture.merge(config, {
        'database': {'url': urllib.parse.urlunsplit(address._replace(path='/' + database))},
        'web': {'binding': '127.0.0.1', 'port': port, 'middlewares': {'logger': {'enable': False}}},
        'auth': {'client_security_disabled': True}, 'jni': smoke_jni.jni_settings(args.java_home),
        'logger': {'level': 'warn'},
        'key_image': {'dicom_path': str(files), 'temp_directory': str(working / 'spool'),
                      'send_to_store': False, 'interval_ms': 1000},
    })

    def start(export):
        config['key_image']['send_to_store'] = export
        config_fixture.write(working / 'config/app.toml', config)
        child_env = env.copy()
        child_env.update(SUMMER_ENV='dev', RUST_LOG='warn')
        log = (OUT / f'app-{export}.log').open('wb')
        try:
            binary = ROOT / 'target/debug' / ('viewer-app.exe' if os.name == 'nt' else 'viewer-app')
            child = subprocess.Popen([str(binary)], cwd=working,
                                     env=child_env, stdout=log, stderr=log, **process_options())
        finally:
            log.close()
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            assert child.poll() is None, 'QA test app exited; see app log'
            try:
                if request(base + '/ready')[0] == 200:
                    return child
            except (OSError, urllib.error.URLError):
                pass
            time.sleep(.1)
        child.kill()
        child.wait()
        raise TimeoutError('QA app readiness')

    try:
        process = start(False)
        for case_index, case in enumerate(['jpeg', 'raw-pixel']):
            row = next(x for x in fixture if x['case'] == case)
            study, series, sop = ['2.25.' + str(uuid.uuid4().int) for _ in range(3)]
            if args.reference_study:
                study, series = reference, shared_series
            (files / series).mkdir(parents=True, exist_ok=True)
            body = base64.b64decode(row['requestBase64'])
            for old, new in [('1.2.3.4.5', sop), ('1.2.3.4', series), ('1.2.3', study)]:
                body = body.replace(('"' + old + '"').encode(), json.dumps(new).encode())
            if args.reference_study:
                header, tail = body.split(b'\r\n\r\n', 1)
                json_part, remainder = tail.split(b'\r\n--b', 1)
                datasets = json.loads(json_part)
                for dataset in datasets:
                    dataset.update(reference_attributes)
                    dataset['0008103E'] = {'vr': 'LO', 'Value': ['KEY IMAGE - QA TEST']}
                body = header + b'\r\n\r\n' + json.dumps(datasets).encode() + b'\r\n--b' + remainder
            status, _, response = request(base + '/client/link', body=json.dumps({
                'storeUrl': store, 'storeAuth': auth, 'studies': [study], 'expireIn': 10}).encode(),
                media='application/json')
            assert status == 200, 'QA session creation failed'
            session = json.loads(response)['session']['id']
            status, _, response = request(base + f'/v1/session/{session}/studies', body=body,
                                         media='multipart/related; boundary=b')
            entry = {'case': case, 'study': study, 'series': series, 'sop': sop, 'ingestStatus': status}
            result['cases'].append(entry)
            assert status == 200, 'QA ingest failed'
            expected_count = case_index + 1 if args.reference_study else 1
            assert sql(f"SELECT num_of_image || ':' || succeed FROM study_key_image WHERE id='{study}'") == f'{expected_count}:false'
            entry['pendingBeforeExport'] = True
            path = f'/v1/session/{session}/wado-rs/studies/{study}/series/{series}/instances/{sop}/frames/1'
            status, _, data = request(base + path)
            entry['localFrameStatus'] = status
            if case == 'jpeg':
                assert status == 200 and data, 'Local JPEG frame unavailable before export'
            else:
                entry['localFrameLimitation'] = 'Java compressed-frame helper does not support raw pixel data'
            metadata_path = f'/v1/session/{session}/wado-rs/studies/{study}/series/{series}/metadata'
            status, _, local_metadata = request(base + metadata_path)
            assert status == 200 and json.loads(local_metadata), 'Local metadata unavailable'
            entry['localMetadataBeforeExport'] = True
            dicoms = list((files / series).glob(sop + '.dcm'))
            assert len(dicoms) == 1
            shutil.copy2(dicoms[0], OUT / f'{case}-local.dcm')
            request(base + f'/v1/session/{session}', method='DELETE')
        stop(process)
        process = None
        process = start(True)
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline and sql('SELECT count(*) FROM study_key_image') != '0':
            time.sleep(.25)
        pending = sql('SELECT count(*) FROM study_key_image')
        result['pendingAfterExport'] = int(pending)
        result['pendingDiagnostics'] = sql('SELECT num_of_retry, left(description,500) FROM study_key_image')
        for entry in result['cases']:
            study, series, sop, case = [entry[k] for k in ['study', 'series', 'sop', 'case']]
            path = f'{store}/studies/{study}/series/{series}/instances/{sop}'
            readback_deadline = time.monotonic() + 30
            while True:
                status, _, data = request(path + '/metadata', auth, accept='application/dicom+json')
                metadata_rows = json.loads(data) if status == 200 else []
                if isinstance(metadata_rows, list) and metadata_rows:
                    break
                assert time.monotonic() < readback_deadline, 'QA metadata not visible within 30 seconds after STOW'
                time.sleep(.5)
            entry['qaMetadataStatus'] = status
            assert status == 200, 'QA metadata missing after scheduler STOW'
            metadata = metadata_rows[0]
            assert metadata['00080018']['Value'][0] == sop
            status, headers, data = request(path, auth, accept='multipart/related; type="application/dicom"; transfer-syntax=*')
            if status == 406:
                status, headers, data = request(path, auth, accept='application/dicom')
            assert status == 200
            content = next((v for k, v in headers.items() if k.lower() == 'content-type'), '')
            message = BytesParser(policy=default).parsebytes(('Content-Type: ' + content + '\r\nMIME-Version: 1.0\r\n\r\n').encode() + data)
            payload = next(message.iter_parts()).get_payload(decode=True)
            (OUT / f'{case}-qa.dcm').write_bytes(payload)
            local = pydicom.dcmread(OUT / f'{case}-local.dcm')
            remote = pydicom.dcmread(io.BytesIO(payload))
            assert remote.StudyInstanceUID == study and remote.SeriesInstanceUID == series and remote.SOPInstanceUID == sop
            assert remote.file_meta.TransferSyntaxUID == local.file_meta.TransferSyntaxUID
            assert remote.PixelData == local.PixelData
            if case == 'jpeg':
                image = Image.open(io.BytesIO(next(generate_frames(remote.PixelData))))
                image.load()
                assert image.size == (8, 8)
            else:
                assert remote.Rows == remote.Columns == 8 and len(remote.PixelData) == 64
            entry.update(qaReadbackPixelMatch=True, transferSyntax=str(remote.file_meta.TransferSyntaxUID),
                         localFilesCleaned=not (files / series).exists())
        assert pending == '0' and all(x['localFilesCleaned'] for x in result['cases']), 'Scheduler did not clean local pending/files'
        if args.reference_study:
            status, _, data = request(f'{store}/studies/{reference}/series', auth, accept='application/dicom+json')
            assert status == 200
            after_series = {row['0020000E']['Value'][0]: row.get('00201209', {}).get('Value', [None])[0]
                            for row in json.loads(data) if row.get('0020000E', {}).get('Value')}
            assert all(uid in after_series and (count is None or after_series[uid] == count)
                       for uid, count in original_series.items()), 'Original series changed unexpectedly'
            assert shared_series in after_series, 'New key image series not listed'
            result['originalSeriesPreserved'] = True if original_series else None
            result['newSeriesInstanceCount'] = after_series[shared_series]
        result['passed'] = True
        print('PASS: QA STOW/readback for JPEG and raw-pixel, pending/restart/local WADO/cleanup')
    finally:
        if process is not None and process.poll() is None:
            stop(process)
        sql(f'DROP DATABASE {database} WITH (FORCE)', address.path.lstrip('/'))
        result['isolatedDatabaseDropped'] = True
        # Remove only the generated configuration containing local credentials.
        (working / 'config/app.toml').unlink(missing_ok=True)
        (OUT / 'result.json').write_text(json.dumps(result, indent=2), encoding='utf-8')


if __name__ == '__main__':
    main()
