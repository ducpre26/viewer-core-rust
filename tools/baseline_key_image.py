"""Capture key image ingest behavior without changing the Java source snapshot."""
from pathlib import Path
import os
import subprocess
import zipfile
import argparse

ROOT = Path(__file__).resolve().parents[1]
JAVA = ROOT.parent / '1pacs-viewer-core'
JDK = Path('C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot')
OUT = ROOT / 'target/step-13-logs'

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java-home',type=Path,default=JDK)
    args=parser.parse_args()
    jdk=args.java_home.resolve()
    suffix='.exe' if os.name=='nt' else ''
    revision=subprocess.check_output(['git','-c',f'safe.directory={JAVA.as_posix()}','-C',str(JAVA),'rev-parse','HEAD'],text=True).strip()
    if revision!='2373896fca106bab446c0e14bb38f0b8d1725949':
        raise RuntimeError('Java revision changed; review Step 13 contract before replacing fixtures')
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
    sources = [ROOT/'tools/Step13Baseline.java']
    sources += list((JAVA/'src/main/java/vn/saolasoft/viewer/module/stowrs/service').glob('*.java'))
    sources += list((JAVA/'src/main/java/vn/saolasoft/viewer/module/keyimage/service').glob('*.java'))
    with (OUT/'java-baseline.log').open('w',encoding='utf-8') as log:
        subprocess.run([str(jdk/'bin'/f'javac{suffix}'),'-proc:none','-cp',cp,'-d',str(OUT),*map(str,sources)],stdout=log,stderr=subprocess.STDOUT,check=True)
        subprocess.run([str(jdk/'bin'/f'java{suffix}'),f'-Dorg.bytedeco.javacpp.cachedir={OUT / "javacpp"}','--enable-native-access=ALL-UNNAMED','-cp',cp,'Step13Baseline',str(ROOT/'docs/baseline/step-13-java.json'),str(OUT/'files')],stdout=log,stderr=subprocess.STDOUT,check=True)
    print('PASS: docs/baseline/step-13-java.json (Java services; repository/session doubles, no Spring container/real DB)')

if __name__ == '__main__':
    main()
