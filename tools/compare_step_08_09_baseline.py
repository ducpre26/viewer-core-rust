"""Compare exported Java/Rust Step 8/9 fixtures; never equate export success with parity.

Run from the Rust repository after the Java Step0809BaselineTest and Rust
step_08_09_baseline_export tests. Raw artifacts remain under each repo's target/.
"""
import json,re,copy
from pathlib import Path
from datetime import datetime
java=json.loads((Path('../1pacs-viewer-core/target/step-08-09-java-logs/baseline-java.json')).read_text())
rust=json.loads(Path('target/step-09-logs/baseline-rust.json').read_text())
uuid=re.compile(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}')
def norm(v,key='',captured=''):
 if isinstance(v,dict):
  out={k:norm(x,k,captured) for k,x in v.items()}
  # Bean Validation does not guarantee iteration order for multiple violations.
  if out.get('errorCode')=='VALIDATION_ERROR' and isinstance(out.get('message'),str):
   out['message']='; '.join(sorted(out['message'].split('; ')))
  return out
 if isinstance(v,list):
  n=[norm(x,key,captured) for x in v]
  return sorted(n,key=lambda x:json.dumps(x,sort_keys=True)) if key in ['studies','permissions','dbStudies'] else n
 if isinstance(v,str):
  if key in ['timestamp','datetime']:return '<generated-time>'
  if key=='expiredTime' and not v.startswith('2030-'):
   return {'ttlMinutes':round((datetime.fromisoformat(v.replace('Z','+00:00'))-datetime.fromisoformat(captured.replace('Z','+00:00'))).total_seconds()/60)}
  if key=='shareKey' and re.fullmatch(r'\d{6}',v):return '<six-digit-key>'
  return uuid.sub('<uuid>',v)
 return v
def convert(row):
 raw=row['body']
 try:body=json.loads(raw)
 except ValueError:body=raw
 return dict(status=row['status'],body=norm(body,captured=row['capturedAt']),dbRows=row['dbRows'],dbStudies=norm(row['dbStudies'],'dbStudies'),dbSessions=norm(row['dbSessions'],'dbSessions',row['capturedAt']))
# User-approved 2026-10-27: parser messages can differ from Java but must
# identify the problem; status/errorCode and all other output still must match.
message_contract = {
 'link-wrong-type': "Field 'studies' must be an array of strings",
 'link-null-body': 'Request body must be a JSON object',
 'update-null-body': 'Request body must be a JSON object',
 'share-null-body': 'Request body must be a JSON object',
}
assert len(java)==len(rust), 'Java/Rust export fixture counts differ'
results=[]
for j,r in zip(java,rust):
 assert j['name']==r['name']
 a,b=convert(j),convert(r)
 accepted_message=False
 if j['name'] in message_contract and isinstance(b['body'],dict):
  message=b['body'].get('message','')
  left,right=copy.deepcopy(a),copy.deepcopy(b)
  if isinstance(left['body'],dict): left['body'].pop('message',None)
  right['body'].pop('message',None)
  accepted_message=message.startswith(message_contract[j['name']]) and left==right
 results.append(dict(name=j['name'],step=j['step'],match=a==b,contractMatch=a==b or accepted_message,acceptedMessage=accepted_message and a!=b,java=a,rust=b))
Path('target/step-09-logs/comparison.json').write_text(json.dumps(results,indent=2)+'\n')
for step in [8,9]:
 rows=[r for r in results if r['step']==step];print('Step',step, 'match',sum(r['match'] for r in rows),'/',len(rows),'contract',sum(r['contractMatch'] for r in rows),'/',len(rows))
for r in results:
 if not r['match']:print(json.dumps(r,ensure_ascii=False))

# Nonzero means outputs differ, even though both export tests may pass.
raise SystemExit(1 if any(not r['contractMatch'] for r in results) else 0)
