"""Step 17. Exercise real HTTP/PostgreSQL/JNI against an independent DIMSE SCP."""
import argparse
import http.client
import io
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
import threading
from concurrent.futures import ThreadPoolExecutor

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "target/step-17-tools"))
from PIL import Image
from pydicom.dataset import Dataset
from pydicom.sequence import Sequence
from pydicom.uid import generate_uid
from pynetdicom import AE, evt
from pynetdicom.sop_class import BasicGrayscalePrintManagementMeta, BasicColorPrintManagementMeta
import config_fixture
import smoke_jni
from smoke_app import stop
from test_download_failures import process_options


def free_port():
    with socket.socket() as connection:
        connection.bind(("127.0.0.1", 0))
        return connection.getsockname()[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-home", type=Path, required=True)
    parser.add_argument("--java-baseline", type=Path, help="Original Java target/classes for the raster baseline probe")
    args = parser.parse_args()
    logs = ROOT / "target/step-17-logs"
    logs.mkdir(exist_ok=True)
    import logging
    logging.basicConfig(filename=logs / "simulator.log",level=logging.INFO,force=True)
    env = smoke_jni.environment(args.java_home)
    if args.java_baseline:
        distribution = ROOT / "java/dicom-facade/target/distribution"
        classpath = os.pathsep.join(map(str,[args.java_baseline.resolve(),distribution / "pacs-dicom-facade.jar",*sorted((distribution / "dependencies").glob("*.jar"))]))
        picture = logs / "baseline.png"
        Image.new("RGB",(2,2),(100,150,200)).save(picture)
        output = subprocess.check_output([str(args.java_home / "bin/java"),"-cp",classpath,
            str(ROOT / "java/dicom-facade/src/smoke/java/PrintBaseline.java"),str(picture)],env=env,text=True,timeout=30)
        assert output.splitlines() == ["grayscale=4","color=0"], output
        (logs / "java-baseline.txt").write_text(output)

    baseline = json.loads((ROOT / "docs/baseline/step-17-java.json").read_text())
    events = []
    state = {"mode": "success", "color": False}

    def create(event):
        events.append("N-CREATE")
        result = Dataset()
        if str(event.request.AffectedSOPClassUID) == "1.2.840.10008.5.1.1.2":
            image = Dataset()
            image.ReferencedSOPClassUID = "1.2.840.10008.5.1.1.4" if event.context.abstract_syntax == BasicGrayscalePrintManagementMeta else "1.2.840.10008.5.1.1.4.1"
            image.ReferencedSOPInstanceUID = generate_uid()
            result.ReferencedImageBoxSequence = Sequence([image])
        return 0, result

    def set_image(event):
        events.append("N-SET")
        data = event.modification_list
        image = data.get("BasicColorImageSequence", data.get("BasicGrayscaleImageSequence"))[0]
        assert image.Rows == 2 and image.Columns == 2
        assert image.SamplesPerPixel in [1, 3]
        if state["color"]:
            # The Java baseline handles DataBufferByte/Short/UShort only. Its PNG
            # RGB decoder returns DataBufferInt, leaving Pixel Data empty.
            assert (len(image.PixelData) if image.PixelData else 0) == baseline["observedPixelBytes"]["color"]
            return 0xC000, None
        assert len(image.PixelData) == baseline["observedPixelBytes"]["grayscale"]
        return 0, None

    def action(event):
        events.append("N-ACTION")
        if state["mode"] == "concurrent":
            state["barrier"].wait(timeout=4)
        if state["mode"] == "timeout":
            time.sleep(7)
        return ({"failure":0xC000,"warning":0xB600}.get(state["mode"],0)), None

    def delete(event):
        events.append("N-DELETE")
        return 0

    ae = AE(ae_title="TEST_PRINTER")
    ae.maximum_associations = 32  # Test SCP must accept all concurrent fixture calls.
    ae.require_called_aet = True
    ae.require_calling_aet = ["TEST_VIEWER"]
    ae.add_supported_context(BasicGrayscalePrintManagementMeta)
    ae.add_supported_context(BasicColorPrintManagementMeta)
    server = ae.start_server(("127.0.0.1", free_port()), block=False, evt_handlers=[
        (evt.EVT_N_CREATE, create), (evt.EVT_N_SET, set_image),
        (evt.EVT_N_ACTION, action), (evt.EVT_N_DELETE, delete)])
    printer_port = server.server_address[1]
    results = []
    with tempfile.TemporaryDirectory(prefix="print-http-", dir=ROOT / "target") as temporary:
        working = Path(temporary)
        port = free_port()
        settings = config_fixture.merge(config_fixture.settings(), {
            "auth": {"client_security_disabled": True},
            "jni": smoke_jni.jni_settings(args.java_home),
            "web": {"binding": "127.0.0.1", "port": port, "global_prefix": "/ws/rest", "graceful": True},
            "logger": {"level": "warn"}})
        config_fixture.write(working / "config/app.toml", settings)
        def request(path, method="GET", body=None, media="application/json", key=None):
            connection = http.client.HTTPConnection("127.0.0.1", port, timeout=20)
            headers = {"Content-Type": media}
            if key is not None:
                headers["session-key"] = key
            try:
                connection.request(method, "/ws/rest" + path, body, headers)
                response = connection.getresponse()
                raw = response.read()
                try:
                    parsed = json.loads(raw) if raw else None
                except json.JSONDecodeError:
                    parsed = raw.decode(errors="replace")
                return response.status, parsed
            finally:
                connection.close()
        def upload(path, color=False, image_bytes=None, duplicate_image=False):
            picture = io.BytesIO()
            Image.new("RGB", (2,2), (100,150,200)).save(picture, format="PNG")
            fields = {"printerId": str(printer_id), "colorPrint": str(color).lower()}
            data = b"".join(b"--print-test\r\nContent-Disposition: form-data; name=\"" + name.encode() + b"\"\r\n\r\n" + value.encode() + b"\r\n" for name,value in fields.items())
            data += b'--print-test\r\nContent-Disposition: form-data; name="image"; filename="image.png"\r\nContent-Type: image/png\r\n\r\n' + (picture.getvalue() if image_bytes is None else image_bytes) + b"\r\n--print-test--\r\n"
            if duplicate_image:
                extra = b'--print-test\r\nContent-Disposition: form-data; name="image"; filename="second.png"\r\nContent-Type: image/png\r\n\r\nmalformed-second-image\r\n'
                data = data.removesuffix(b"--print-test--\r\n") + extra + b"--print-test--\r\n"
            return request(path,"POST",data,"multipart/form-data; boundary=print-test")
        binary = ROOT / "target/debug" / ("viewer-app.exe" if os.name == "nt" else "viewer-app")
        subprocess.run(["cargo","build","--locked","-p","viewer-app"],cwd=ROOT,env=env,check=True,timeout=240)
        env.update(SUMMER_ENV="dev", RUST_LOG="warn")
        with (logs / "app.log").open("wb") as log:
            app = subprocess.Popen([str(binary)],cwd=working,env=env,stdout=log,stderr=log,**process_options())
            (logs / "pid.txt").write_text(str(app.pid))
            session = None
            printer_id = None
            try:
                deadline = time.monotonic() + 40
                while True:
                    assert app.poll() is None, "Application exited; see app.log"
                    try:
                        if request("/ready")[0] == 200: break
                    except OSError: pass
                    assert time.monotonic() < deadline, "Readiness timeout"
                    time.sleep(.05)
                status, body = request("/client/link","POST",json.dumps({"storeUrl":"http://127.0.0.1:9/wado-rs","studies":["2.25.17"],"expireIn":10}))
                assert status == 200, body
                session = body["session"]["id"]
                base = f"/v1/session/{session}"
                printer = {"printerName":"step17-synthetic","host":"127.0.0.1","port":printer_port,"aeTitle":"TEST_PRINTER","calledAeTitle":"TEST_VIEWER","enabled":True}
                status, body = request(base + "/printer","POST",json.dumps(printer))
                assert status == 201, body
                printer_id = body["body"]
                assert request(base + f"/printer/{printer_id}")[1]["body"]["aeTitle"] == "TEST_PRINTER"
                assert request(base + "/printer?offset=0&limit=100")[1]["header"]["code"] == 302
                for color in [False,True]:
                    events.clear()
                    state["color"] = color
                    status, body = upload(base + "/print",color)
                    assert status == 200 and body["body"]["succeed"] == (not color), body
                    expected = ["N-CREATE","N-CREATE","N-SET"]
                    if not color: expected += ["N-ACTION","N-DELETE","N-DELETE"]
                    assert events == expected, events
                    results.append("color-PNG-Java-compatibility" if color else "grayscale")
                state["color"] = False
                status, body = upload(base + "/print",duplicate_image=True)
                assert status == 200 and body["body"]["succeed"], body
                results.append("duplicate-image-first-file")
                state["color"] = True
                assert not upload(base + "/print?colorPrint=on")[1]["body"]["succeed"]
                results.append("form-query-binding")
                state["color"] = False
                for mode in ["warning","failure","timeout"]:
                    state["mode"] = mode
                    events.clear()
                    status, body = upload(base + "/print")
                    assert status == 200 and body["body"]["succeed"] == (mode == "warning"), body
                    assert events.count("N-ACTION") == 1, events
                    results.append(mode)
                    if mode == "timeout":
                        assert "outcome_unknown" in body["body"]["message"], body
                        time.sleep(3)
                state["mode"] = "concurrent"
                state["barrier"] = threading.Barrier(12)
                with ThreadPoolExecutor(max_workers=12) as callers:
                    concurrent = list(callers.map(lambda _: upload(base + "/print"),range(12)))
                for status,body in concurrent:
                    assert status == 200 and body["body"]["succeed"], (status,body)
                results.append("concurrent-12-print-transactions")
                state["mode"] = "success"
                assert upload(base + "/print",image_bytes=b"x" * (9 * 1024 * 1024))[0] == 200
                status, body = upload(base + "/print",image_bytes=b"x" * (11 * 1024 * 1024))
                assert status == 500 and body["errorCode"] == "INTERNAL_ERROR", (status,body)
                results.append("multipart-request-limit")
                for field,value in [("aeTitle","WRONG_PRINTER"),("port",free_port())]:
                    request(base + "/printer","PUT",json.dumps({"id":printer_id,field:value}))
                    events.clear()
                    assert not upload(base + "/print")[1]["body"]["succeed"]
                    assert not events
                    request(base + "/printer","PUT",json.dumps({"id":printer_id,field:printer[field]}))
                for field,value in [("port",0),("port",65536),("printerName",""),("host","x"*256)]:
                    invalid = {**printer,field:value}
                    assert request(base + "/printer","POST",json.dumps(invalid))[0] == 400
                assert request(base + "/printer","PUT",json.dumps({"id":printer_id,"description":None}))[0] == 200
                assert request(base + "/printer?limit=0")[0] == 400
                assert request(base + "/printer/2147483647")[0] == 404
                assert request(base + "/printer","PUT",json.dumps({"id":2147483647}))[0] == 404
                assert request(base + "/print","POST",b"--x--\r\n","multipart/form-data; boundary=x")[0] == 400
                results += ["association-reject","offline","validation","missing-printer"]
                assert not upload(base + "/print",image_bytes=b"malformed")[1]["body"]["succeed"]
                request(base + "/printer","PUT",json.dumps({"id":printer_id,"enabled":False}))
                events.clear()
                assert upload(base + "/print")[1]["body"]["message"] == "Printer not found or not enabled"
                assert not events
                assert request("/v1/session/nonexistent/print","POST",b"", "multipart/form-data; boundary=x")[0] == 401
                assert request(base + f"/printer/{printer_id}","DELETE")[0] == 200
                assert request(base + f"/printer/{printer_id}","DELETE")[0] == 404
                printer_id = None
                results += ["malformed","disabled","session-auth","crud"]
            finally:
                if app.poll() is None:
                    try:
                        if session:
                            base = f"/v1/session/{session}"
                            if printer_id: request(base + f"/printer/{printer_id}","DELETE")
                            request(base,"DELETE")
                    finally:
                        stop(app)
                server.shutdown()
    (logs / "result.json").write_text(json.dumps(results,indent=2))
    print("PASS: " + ", ".join(results))

if __name__ == "__main__":
    main()
