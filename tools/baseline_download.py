"""Capture download behavior without changing the Java source snapshot."""
from pathlib import Path
import os
import subprocess
import zipfile
import argparse

ROOT = Path(__file__).resolve().parents[1]
JAVA = ROOT.parent / '1pacs-viewer-core'
JDK = Path('C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot')
OUT = ROOT / 'target/step-12-logs'

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mp4-directory',type=Path,help='Synthetic codec output directory containing input.dcm')
    parser.add_argument('--java-home',type=Path,default=JDK)
    args=parser.parse_args()
    jdk=args.java_home.resolve()
    suffix='.exe' if os.name=='nt' else ''
    OUT.mkdir(parents=True, exist_ok=True)
    libs = OUT / 'baseline-libs'
    libs.mkdir(exist_ok=True)
    jar = JAVA / 'target/saola-viewer-core-2.2.4-pre.jar'
    with zipfile.ZipFile(jar) as archive:
        for name in archive.namelist():
            if name.startswith('BOOT-INF/lib/') and name.endswith('.jar'):
                (libs / Path(name).name).write_bytes(archive.read(name))
    # Test-only Mockito/Spring libraries are not in the application distribution.
    cache = Path.home() / '.m2/repository'
    for group in ['org/mockito/mockito-core','net/bytebuddy/byte-buddy','net/bytebuddy/byte-buddy-agent','org/objenesis/objenesis','org/springframework/spring-test','org/projectlombok/lombok']:
        candidates = sorted(p for p in (cache/group).rglob('*.jar') if not any(x in p.name for x in ['-sources','-javadoc']))
        source = candidates[-1]
        target = libs / source.name
        target.write_bytes(source.read_bytes())
    cp = os.pathsep.join(map(str,[OUT, JAVA/'target/classes',*sorted(libs.glob('*.jar'))]))
    stub = OUT / 'OrthancImageRenderer.java'
    stub.write_text('package org.orthanc.transcode; public class OrthancImageRenderer { public static byte[] renderImage(byte[] b, int f, ImageFormat t, int q) { return new byte[]{(byte)255,(byte)f,42}; } }')
    sources = [ROOT/'tools/Step12Baseline.java', stub]
    sources += list((JAVA/'src/main/java/vn/saolasoft/viewer/module/file/service').glob('*.java'))
    sources += [JAVA/'src/main/java/vn/saolasoft/viewer/module/file/rest/DownloadAPI.java']
    with (OUT/'java-baseline.log').open('w',encoding='utf-8') as log:
        subprocess.run([str(jdk/'bin'/f'javac{suffix}'),'-proc:none','-cp',cp,'-d',str(OUT),*map(str,sources)],stdout=log,stderr=subprocess.STDOUT,check=True)
        subprocess.run([str(jdk/'bin'/f'java{suffix}'),f'-Dorg.bytedeco.javacpp.cachedir={OUT / "javacpp"}','--enable-native-access=ALL-UNNAMED','-cp',cp,'Step12Baseline',str(ROOT/'docs/baseline/step-12-java.json'),*([str(args.mp4_directory.resolve())] if args.mp4_directory else [])],stdout=log,stderr=subprocess.STDOUT,check=True)
    print('PASS: docs/baseline/step-12-java.json (synthetic contract; not codec verification)')

if __name__ == '__main__':
    main()
