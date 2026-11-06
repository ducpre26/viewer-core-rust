"""Direct STOW/readback of the existing reference-study key images. No Rust/DB/JVM."""
import argparse
import base64
import json
from pathlib import Path
import time
import urllib.error
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--store-file', type=Path, default=ROOT / 'docs/STORE_2.md')
    parser.add_argument('--source', type=Path, default=ROOT / 'target/step-13-logs/store-2-reference-stow')
    parser.add_argument('--output', type=Path, default=ROOT / 'target/step-13-logs/direct-store-2')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    lines = args.store_file.read_text(encoding='utf-8').splitlines()
    urls = [line.split() for line in lines if line.startswith('URL ')]
    wado = next(parts[2] for parts in urls if len(parts) == 3 and parts[1].lower() == 'wadors').rstrip('/')
    stow = next(parts[2] for parts in urls if len(parts) == 3 and parts[1].lower() == 'stowrs').rstrip('/')
    study = next(line[6:].strip() for line in lines if line.startswith('SIUID '))
    auth = next(line[5:].strip() for line in lines if line.startswith('auth '))
    if not auth.lower().startswith(('bearer ', 'basic ')):
        user, password = (part.strip() for part in auth.split('/', 1))
        auth = 'Basic ' + base64.b64encode(f'{user}:{password}'.encode()).decode()
    source = json.loads((args.source / 'result.json').read_text())
    assert all(entry['study'] == study for entry in source['cases']), 'Source study differs from STORE_2.md'

    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, *args, **kwargs):
            return None

    http = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())

    def request(url, data=None, media=None, accept='application/dicom+json'):
        headers = {'Authorization': auth, 'Accept': accept}
        if media:
            headers['Content-Type'] = media
        req = urllib.request.Request(url, data=data, headers=headers)
        try:
            response = http.open(req, timeout=20)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            return response.status, response.headers.get('Content-Type', ''), response.read()

    boundary = 'direct-' + uuid.uuid4().hex
    parts = []
    for entry in source['cases']:
        payload = (args.source / (entry['case'] + '-local.dcm')).read_bytes()
        parts.append(('--' + boundary + '\r\nContent-Type: application/dicom\r\n\r\n').encode() + payload + b'\r\n')
    body = b''.join(parts) + ('--' + boundary + '--\r\n').encode()
    status, media, response = request(stow, body, f'multipart/related; type="application/dicom"; boundary={boundary}')
    (args.output / 'stow-response.body').write_bytes(response)
    result = {'directToStore': True, 'rustBackendUsed': False, 'metadataJsonIncluded': False,
              'stowUrl': stow, 'study': study, 'sameExistingSopUids': True,
              'stowStatus': status, 'stowContentType': media, 'cases': [], 'readbackVerified': False}
    try:
        result['stowResponse'] = json.loads(response)
    except ValueError:
        result['stowResponseIsJson'] = False
    deadline = time.monotonic() + 30
    while True:
        entries = []
        for entry in source['cases']:
            path = wado + f"/studies/{study}/series/{entry['series']}/instances/{entry['sop']}"
            code, content_type, data = request(path + '/metadata')
            (args.output / (entry['case'] + '-metadata.body')).write_bytes(data)
            try:
                metadata = json.loads(data)
                count = len(metadata) if isinstance(metadata, list) else None
            except ValueError:
                count = None
            entries.append({'case': entry['case'], 'series': entry['series'], 'sop': entry['sop'],
                            'metadataStatus': code, 'metadataItems': count})
        if all(entry['metadataStatus'] == 200 and entry['metadataItems'] for entry in entries) or time.monotonic() >= deadline:
            break
        time.sleep(2)
    for entry in entries:
        path = wado + f"/studies/{study}/series/{entry['series']}/instances/{entry['sop']}"
        code, content_type, data = request(path, accept='application/dicom')
        (args.output / (entry['case'] + '-dicom.body')).write_bytes(data)
        entry.update(dicomStatus=code, dicomContentType=content_type, dicomResponseBytes=len(data))
    result['cases'] = entries
    (args.output / 'result.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
    print(json.dumps({'stowStatus': status, 'cases': entries, 'evidence': str(args.output)}))


if __name__ == '__main__':
    main()
