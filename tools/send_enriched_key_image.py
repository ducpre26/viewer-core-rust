"""Direct STOW of one JPEG key image with attributes copied from source instance.

Source metadata and outputs stay in ignored target/. No Rust/JVM/DB calls.
"""
import io
import json
from pathlib import Path
import sys
import time
import urllib.error
import urllib.request
import uuid
from email.parser import BytesParser
from email.policy import default

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'target/step-12-tools'))
import pydicom


def main():
    out = ROOT / 'target/step-13-logs/enriched-direct-store-2'
    out.mkdir(parents=True, exist_ok=True)
    assert not (out / 'result.json').exists(), 'Existing upload evidence; do not create duplicate test images'
    lines = (ROOT / 'docs/STORE_2.md').read_text().splitlines()
    urls = [line.split() for line in lines if line.startswith('URL ')]
    wado = next(x[2] for x in urls if x[1].lower() == 'wadors').rstrip('/')
    stow = next(x[2] for x in urls if x[1].lower() == 'stowrs').rstrip('/')
    study = next(line[6:].strip() for line in lines if line.startswith('SIUID '))
    auth = next(line[5:].strip() for line in lines if line.startswith('auth '))
    metadata = json.loads((ROOT / 'target/step-13-logs/key-image-source/original-instances.json').read_text())[0]
    assert metadata['0020000D']['Value'][0] == study
    required = {'PatientID': '00100020', 'PatientName': '00100010', 'StudyDate': '00080020',
                'StudyTime': '00080030', 'AccessionNumber': '00080050', 'Modality': '00080060'}
    assert all(tag in metadata for tag in required.values()), 'Source is missing a requested tag'
    assert all(metadata[tag].get('Value') for name, tag in required.items() if name != 'AccessionNumber')
    selected = {tag: value for tag, value in metadata.items()
                if tag.startswith('0010') or tag in {
                    '00080005', '00080020', '00080030', '00080050', '00080060',
                    '00080080', '00080090', '00081030', '00200010'}}
    source_attributes = pydicom.Dataset.from_json(selected)
    ds = pydicom.dcmread(ROOT / 'target/step-13-logs/store-2-reference-stow/jpeg-local.dcm')
    for element in source_attributes:
        ds.add(element)
    ds.StudyInstanceUID = study
    ds.SeriesInstanceUID = '2.25.' + str(uuid.uuid4().int)
    ds.SOPInstanceUID = '2.25.' + str(uuid.uuid4().int)
    ds.file_meta.MediaStorageSOPInstanceUID = ds.SOPInstanceUID
    ds.SeriesDescription = 'KEY IMAGE - QA TEST WITH SOURCE METADATA'
    path = out / 'jpeg-enriched.dcm'
    ds.save_as(path, enforce_file_format=True)
    local = pydicom.dcmread(path)
    assert all(local[tag] == source_attributes[tag] for tag in required.values())
    (out / 'jpeg-enriched.dicom.json').write_text(local.to_json(), encoding='utf-8')
    result = {'directToStore': True, 'rustBackendUsed': False, 'study': study,
              'series': str(local.SeriesInstanceUID), 'sop': str(local.SOPInstanceUID),
              'sourceSop': metadata['00080018']['Value'][0],
              'copiedFields': list(required),
              'sourceAccessionNumberEmpty': not metadata['00080050'].get('Value'),
              'newSeriesAndSopUid': True, 'readbackVerified': False}
    (out / 'result.json').write_text(json.dumps(result, indent=2))

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
            response = http.open(req, timeout=15)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            return response.status, response.headers.get('Content-Type', ''), response.read()

    boundary = 'key-image-' + uuid.uuid4().hex
    body = ('--' + boundary + '\r\nContent-Type: application/dicom\r\n\r\n').encode() + path.read_bytes() + ('\r\n--' + boundary + '--\r\n').encode()
    (out / 'one-key-image.multipart').write_bytes(body)
    try:
        status, media, data = request(stow, body, f'multipart/related; type="application/dicom"; boundary={boundary}')
        result['stowStatus'] = status
        (out / 'stow-response.json').write_bytes(data)
        assert 200 <= status < 300, 'STOW rejected enriched key image'
        url = wado + f'/studies/{study}/series/{local.SeriesInstanceUID}/instances/{local.SOPInstanceUID}'
        deadline = time.monotonic() + 45
        while True:
            code, content_type, data = request(url, accept='application/dicom')
            if code == 200 and data:
                break
            assert time.monotonic() < deadline, 'Enriched DICOM readback not available after 45 seconds'
            time.sleep(1)
        result['dicomReadbackStatus'] = code
        if content_type.startswith('multipart/'):
            message = BytesParser(policy=default).parsebytes(('Content-Type: ' + content_type + '\r\nMIME-Version: 1.0\r\n\r\n').encode() + data)
            data = next(message.iter_parts()).get_payload(decode=True)
        (out / 'jpeg-enriched-readback.dcm').write_bytes(data)
        remote = pydicom.dcmread(io.BytesIO(data))
        assert remote.StudyInstanceUID == study and remote.SeriesInstanceUID == local.SeriesInstanceUID and remote.SOPInstanceUID == local.SOPInstanceUID
        assert remote.PixelData == local.PixelData
        assert remote.file_meta.TransferSyntaxUID == local.file_meta.TransferSyntaxUID
        matches = {name: tag in remote and remote[tag].value == local[tag].value for name, tag in required.items()}
        result['readbackFieldMatches'] = matches
        assert all(matches.values()), 'Readback fields differ from original source'
        result['readbackVerified'] = True
        print('PASS: direct enriched STOW and DICOM readback; all six source fields and pixels match')
    finally:
        (out / 'result.json').write_text(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
