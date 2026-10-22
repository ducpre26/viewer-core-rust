"""Capture source evidence; deliberately not a Spring runtime/OpenAPI export."""
import hashlib
import json
import re
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
JAVA = ROOT.parent / '1pacs-viewer-core'
OUT = ROOT / 'docs/baseline'
OUT.mkdir(parents=True, exist_ok=True)

def write(name, value):
    (OUT / name).write_text(value, encoding='utf-8')

mapping = {'ClientAPI': 1, 'SessionAPI': 2, 'StudyAPI': 3,
           'WadoRsAPI': 4, 'DownloadAPI': 5, 'StowRSAPI': 6,
           'Session3DAPI': 7, 'ConfigPublicAPI': 8, 'AdminAPI': 9,
           'InferenceAPI': 10, 'SegmentationAPI': 11, 'ModelController': 12,
           'PrinterAPI': 13, 'LicenseStatusAPI': 14, 'LicenseCcuAPI': 14}
rows = []
for file in sorted((JAVA / 'src/main/java').rglob('*.java')):
    source = file.read_text(encoding='utf-8')
    source = re.sub(r'/\*.*?\*/|//[^\n]*', lambda m: '\n' * m[0].count('\n'), source, flags=re.S)
    if not re.search(r'^\s*@RestController\s*$', source, re.M):
        continue
    head, body = source.split('public class ', 1)
    prefix = re.search(r'@RequestMapping\("([^"]+)"\)', head)
    if not prefix:
        raise ValueError(f'Unrecognized class mapping: {file}')
    matches = list(re.finditer(r'@(Get|Post|Put|Delete|Patch|Request)Mapping\b(?:\(([^\n]*)\))?', body))
    for i, match in enumerate(matches):
        attrs = match[2] or ''
        path = re.search(r'(?:^\s*|(?:value|path)\s*=\s*)"([^"]*)"', attrs)
        route = prefix[1] + (path[1] if path else '')
        block = body[match.end():matches[i+1].start() if i+1 < len(matches) else len(body)]
        method = re.search(r'public\s+[^\n]*?\b(\w+)\s*\(', block)
        if not method or match[1] == 'Request':
            raise ValueError(f'Unrecognized method: {file}:{route}')
        uc = mapping.get(file.stem)
        if file.stem == 'AdminAPI' and '/config' in route:
            uc = 8
        excluded = '/seal/' in file.as_posix()
        if not uc and not excluded:
            raise ValueError(f'Unmapped controller {file}')
        permission = re.findall(r'@PreAuthorize\(([^\n]+)\)', head + block.split('public ', 1)[0])
        services = sorted(set(re.findall(r'\b(\w+(?:Service|Guard))\.', block)))
        rows.append({'method': match[1].upper(), 'route': route,
                     'controller': file.stem + '.' + method[1],
                     'source': file.relative_to(JAVA).as_posix(),
                     'use_case': f'UC-{uc:03}' if uc else 'excluded:seal',
                     'service_calls': services, 'preauthorize': permission,
                     'entity_trace': 'pending service/repository review',
                     'evidence': 'source-only; runtime verification pending'})
write('endpoint-inventory.json', json.dumps(rows, ensure_ascii=False, indent=2) + '\n')
table = ['# Source endpoint inventory', '',
         'Draft extracted from controller annotations. Not a Spring mapping export. '
         'Service calls are direct receivers only; entity trace and filter authorization need review. '
         'No method annotation does not imply public access. Framework routes and WebSocket are separate.', '',
         '| Method | Route | Handler | UC / exclusion | Direct services | Method authorization |',
         '|---|---|---|---|---|---|']
for r in rows:
    table.append('| ' + ' | '.join([r['method'], '`'+r['route']+'`', r['controller'], r['use_case'],
                                  ', '.join(r['service_calls']), '; '.join(r['preauthorize']).replace('|', '\\|') or 'See WebRequestFilter']) + ' |')
table += ['', '## Additional surfaces requiring runtime reconciliation', '',
          '- `/ws/*`: WebSocketConfig → WebSocketHandler; UC-007.',
          '- `/v3/api-docs/**`, Swagger UI, framework error and actuator routes: infrastructure; effective availability requires runtime export.',
          '- QIDO service/Feign mappings are upstream calls, not standalone incoming controller routes.',
          '- Seal routes remain listed with `excluded:seal` for scope reconciliation.']
write('endpoint-inventory.md', '\n'.join(table) + '\n')

files = sorted((JAVA / 'docs/use_cases').glob('UC-*.md'))
files += [JAVA / 'docs/entity_model.md', JAVA / 'docs/use_cases.puml', JAVA / 'db/init.sql']
manifest = []
for file in files:
    relative = file.relative_to(JAVA)
    target = OUT / 'inputs' / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(file, target)
    manifest.append({'source': relative.as_posix(), 'sha256': hashlib.sha256(file.read_bytes()).hexdigest()})
write('input-hashes.json', json.dumps(manifest, indent=2) + '\n')
checklist = ['# UC verification checklist — draft', '',
             'Extracted assertions from UC documents, not observed Java results. Each unchecked item requires concrete synthetic input, fixture ID, expected HTTP/body and relevant DB/file effects before independent execution.', '']
for file in sorted((JAVA / 'docs/use_cases').glob('UC-*.md')):
    checklist += ['## ' + file.stem, '']
    active = False
    for line in file.read_text(encoding='utf-8').splitlines():
        if line.startswith('## '):
            active = line[3:] in ['Main Success Scenario', 'Alternative Flows', 'Postconditions', 'Business Rules']
            if active:
                checklist += ['### ' + line[3:], '']
        elif active and line.strip():
            if line.startswith('### '):
                checklist += ['#### ' + line[4:]]
            elif re.match(r'^(\d+\. |\- )', line):
                checklist += ['- [ ] ' + re.sub(r'^(\d+\. |\- )', '', line)]
            else:
                checklist += [line]
    checklist += ['']
write('uc-checklist.md', '\n'.join(checklist) + '\n')
config_keys = []
for file in sorted((JAVA / 'src/main/resources').glob('*.properties')):
    for line in file.read_text(encoding='utf-8').splitlines():
        if line.strip() and not line.lstrip().startswith(('#', '!')) and '=' in line:
            config_keys.append((file.name, line.split('=', 1)[0].strip()))
write('configuration-keys.md', '# Configuration keys — draft\n\nNames only; values intentionally omitted. Types, defaults, env overrides and operational constraints still require review.\n\n' + '\n'.join('- `' + f + '`: `' + key + '`' for f, key in config_keys) + '\n')
revision = subprocess.check_output(['git', '-C', str(JAVA), 'rev-parse', 'HEAD'], text=True).strip()
write('source-revision.json', json.dumps({'commit': revision, 'branch': subprocess.check_output(
    ['git', '-C', str(JAVA), 'branch', '--show-current'], text=True).strip(),
    'inputs': 'input-hashes.json', 'note': 'UC/entity documents were untracked at capture; snapshots are supplemental, not part of the Java commit.'}, indent=2) + '\n')
print(f'{len(rows)} source routes; {sum(r["use_case"] == "excluded:seal" for r in rows)} excluded; {len(files)} input snapshots')
