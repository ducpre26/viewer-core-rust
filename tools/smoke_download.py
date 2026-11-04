"""Step 12: real metadata/MP4 smoke and HTTP contract with rendered JPEG store fixtures.

--app runs PostgreSQL/HTTP tests; the codec fixture is explicitly test-only.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import sys
import time
import base64
import config_fixture
import smoke_jni

ROOT = Path(__file__).resolve().parents[1]
DIST = ROOT / 'java/dicom-facade/target/distribution'
LOGS = ROOT / 'target/step-12-logs'
sys.path.insert(0,str(ROOT/'target/step-12-tools'))

def run(command, env, name):
    log = LOGS / name
    with log.open('w',encoding='utf-8') as output:
        process = subprocess.Popen(list(map(str,command)),cwd=ROOT,env=env,stdout=output,stderr=subprocess.STDOUT)
        peak=0
        try:
            import psutil
            monitor=psutil.Process(process.pid)
        except ImportError:
            monitor=None
        deadline=time.monotonic()+600
        while process.poll() is None:
            if monitor:
                try: peak=max(peak,monitor.memory_info().rss)
                except psutil.NoSuchProcess: pass
            if time.monotonic()>deadline:
                process.kill();process.wait();raise RuntimeError(f'Timeout: {log}')
            time.sleep(0.02)
    if peak:
        (LOGS/(name+'.rss.json')).write_text(json.dumps({'peakRssBytes':peak,'scope':'single subprocess sampled every 20ms; synthetic workload'}))
    if process.returncode:
        raise RuntimeError(f'Failed: {log}\n{log.read_text(encoding="utf-8",errors="replace")[-3500:]}')
    print(f'PASS: {name}',flush=True)

def fixture(root, java, env):
    """Keep actual metadata code, replace only codec method bodies in a target copy."""
    source = (ROOT/'java/dicom-facade/src/main/java/vn/saolasoft/pacs/dicom/DicomFacade.java').read_text()
    template = (ROOT/'crates/dicom-jvm/tests/fixtures/DicomFacade.java').read_text()
    for method in ['encodeVideo']:
        def bounds(text):
            start = text.index('    public static void '+method+'(')
            opening = text.index('{',start)
            depth = 1
            end = opening+1
            while depth:
                if text[end]=='{': depth += 1
                elif text[end]=='}': depth -= 1
                end += 1
            return start,end
        a,b = bounds(source)
        c,d = bounds(template)
        source = source[:a]+template[c:d]+source[b:]
    (root/'dependencies').mkdir()
    (root/'classes').mkdir()
    file = root/'DicomFacade.java'
    file.write_text(source,encoding='utf-8')
    suffix = '.exe' if os.name=='nt' else ''
    cp = os.pathsep.join(map(str,[DIST/'pacs-dicom-facade.jar',*sorted((DIST/'dependencies').glob('*.jar'))]))
    run([java/'bin'/f'javac{suffix}','-cp',cp,'-d',root/'classes',file],env,'compile-codec-fixture.log')
    jar = root/'pacs-dicom-facade.jar'
    run([java/'bin'/f'jar{suffix}','--create','--file',jar,'-C',root/'classes','.'],env,'jar-codec-fixture.log')
    for dependency in (DIST/'dependencies').glob('*.jar'):
        if dependency.name.startswith(('dcm4che-core-','dcm4che-json-','dcm4che-dict-','jakarta.json-api-','parsson-','slf4j-')):
            shutil.copyfile(dependency,root/'dependencies'/dependency.name)
    manifest = json.loads((DIST/'manifest.json').read_text())
    manifest['artifacts'] = {p.relative_to(root).as_posix():hashlib.sha256(p.read_bytes()).hexdigest()
                             for p in [jar,*sorted((root/'dependencies').glob('*.jar'))]}
    (root/'manifest.json').write_text(json.dumps(manifest),encoding='utf-8')

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java-home',type=Path,required=True)
    parser.add_argument('--app',action='store_true')
    parser.add_argument('--independent',action='store_true',help='Decode MP4 with external FFmpeg, DICOM with pydicom, JPEG with Pillow; requires tools/download-test-requirements.txt')
    args=parser.parse_args()
    LOGS.mkdir(parents=True,exist_ok=True)
    java=args.java_home.resolve();env=smoke_jni.environment(java)
    suffix='.exe' if os.name=='nt' else ''
    outputs=Path(tempfile.mkdtemp(prefix='codecs-',dir=LOGS)).resolve()
    cp=os.pathsep.join(map(str,[outputs,DIST/'pacs-dicom-facade.jar',*sorted((DIST/'dependencies').glob('*.jar'))]))
    run([java/'bin'/f'javac{suffix}','-cp',cp,'-d',outputs,ROOT/'java/dicom-facade/src/smoke/java/DownloadSmoke.java'],env,'compile-download-smoke.log')
    run([java/'bin'/f'java{suffix}','--enable-native-access=ALL-UNNAMED','-Xcheck:jni','-Xmx256m',f'-Djava.io.tmpdir={outputs}',f'-Dorg.bytedeco.javacpp.cachedir={outputs / "javacpp"}','-cp',cp,'DownloadSmoke',outputs],env,'download-codecs.log')
    print(f'Codec outputs: {outputs}; no Orthanc dependency',flush=True)
    (LOGS/'latest-codec-outputs.txt').write_text(str(outputs),encoding='utf-8')
    settings=smoke_jni.jni_settings(java)
    # Large metadata regression crosses both JNI directions; size this test heap accordingly.
    settings["xmx_mb"]=384
    config=LOGS/'jni-real.toml';config_fixture.write(config,settings)
    env['JNI_TEST_CONFIG']=str(config)
    env['JNI_DOWNLOAD_SOURCE']=str(outputs/'input.dcm')
    env['JNI_DOWNLOAD_OUTPUT']=str(outputs/'jni-output.mp4')
    run(['cargo','test','-p','dicom-jvm','--locked','jvm_download_video','--','--ignored','--nocapture'],env,'jni-real-video.log')
    if args.independent:
        import imageio_ffmpeg
        ffmpeg=imageio_ffmpeg.get_ffmpeg_exe()
        decoded=[]
        for name in ['output.mp4','jni-output.mp4']:
            result=subprocess.run([ffmpeg,'-v','error','-i',str(outputs/name),'-map','0:v:0','-f','rawvideo','-pix_fmt','gray','pipe:1'],capture_output=True,check=True,timeout=30)
            assert len(result.stdout)==16*16*2,'Independent MP4 dimensions/frame count'
            decoded.append(result.stdout)
        assert decoded[0]==decoded[1],'Direct Java and JNI decoded pixels match'
        baseline=json.loads((ROOT/'docs/baseline/step-12-media.json').read_text())
        assert (outputs/'input.dcm').read_bytes()==base64.b64decode(baseline['inputDicomBase64']),'Same synthetic DICOM corpus as original Java'
        (outputs/'baseline-java.mp4').write_bytes(base64.b64decode(baseline['mp4Base64']))
        original=subprocess.run([ffmpeg,'-v','error','-i',str(outputs/'baseline-java.mp4'),'-map','0:v:0','-f','rawvideo','-pix_fmt','gray','pipe:1'],capture_output=True,check=True,timeout=30)
        assert original.stdout==decoded[0]==base64.b64decode(baseline['decodedGrayBase64']),'Original Java and migrated pipeline decoded pixels match'
        (LOGS/'mp4-independent.json').write_text(json.dumps({'decoder':ffmpeg,'decodedBytes':len(decoded[0]),'frames':2,'width':16,'height':16,'jniMatchesJava':True,'originalJavaMatches':True}))
        print('PASS independent output decode',flush=True)
    if args.app:
        with tempfile.TemporaryDirectory(prefix='download-fixture-',dir=ROOT/'target') as temporary:
            root=Path(temporary).resolve();fixture(root,java,env)
            settings=smoke_jni.jni_settings(java);settings["facade_dir"]=str(root)
            config=config_fixture.ROOT/'target/step-12-logs/jni-fixture.toml'
            config_fixture.write(config,settings);env['JNI_TEST_CONFIG']=str(config)
            run(['cargo','test','-p','viewer-app','--locked','postgres_download_contract','--','--ignored','--nocapture'],env,'http-download-contract.log')

if __name__=='__main__':
    main()
