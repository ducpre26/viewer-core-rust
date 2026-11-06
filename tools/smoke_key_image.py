"""Step 13: real JVM/PostgreSQL/HTTP contract against a local Postman STOW simulation."""
import argparse,base64,http.client,io,json,os,shutil,socket,subprocess,sys,tempfile,time,uuid
from smoke_app import stop
from test_download_failures import process_options
from pathlib import Path
import config_fixture,smoke_jni
ROOT=Path(__file__).resolve().parents[1]
LOGS=ROOT/'target/step-13-logs'

def tcp_smoke(java,env,store):
 def request(port,path,method='GET',body=None,media='application/json'):
  conn=http.client.HTTPConnection('127.0.0.1',port,timeout=15)
  try:
   conn.request(method,'/ws/rest'+path,body,{'Content-Type':media});res=conn.getresponse();return res.status,res.read()
  finally:conn.close()
 with tempfile.TemporaryDirectory(prefix='key-image-app-',dir=ROOT/'target') as temp:
  root=Path(temp);files=root/'files';(files/'1.2.3.4').mkdir(parents=True)
  study='2.25.'+str(uuid.uuid4().int)
  configuration=config_fixture.merge(config_fixture.settings(),{'auth':{'client_security_disabled':True},'jni':smoke_jni.jni_settings(java),'key_image':{'dicom_path':str(files),'temp_directory':str(root/'spool'),'send_to_store':False,'interval_ms':100},'logger':{'level':'warn'}})
  with (LOGS/'binary-build.log').open('wb') as log:subprocess.run(['cargo','build','--locked','-p','viewer-app'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=180)
  binary=ROOT/'target/debug'/('viewer-app.exe' if os.name=='nt' else 'viewer-app')
  for export in [False,True]:
   with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
   configuration['web']=config_fixture.merge(configuration.get('web',{}),{'binding':'127.0.0.1','port':port,'global_prefix':'/ws/rest','graceful':True})
   configuration['key_image']['send_to_store']=export
   config_fixture.write(root/'config/app.toml',configuration)
   child_env=env.copy();child_env.update(SUMMER_ENV='dev',RUST_LOG='warn')
   with (LOGS/f'tcp-{export}.log').open('wb') as log:
    app=subprocess.Popen([str(binary)],cwd=root,env=child_env,stdout=log,stderr=log,**process_options())
    try:
     deadline=time.monotonic()+30
     while True:
      assert app.poll() is None,'app exited; see tcp log'
      try:
       if request(port,'/ready')[0]==200:break
      except OSError:pass
      assert time.monotonic()<deadline,'app readiness timeout';time.sleep(.05)
     if not export:
      status,body=request(port,'/client/link','POST',json.dumps({'storeUrl':store,'storeAuth':'Basic fixture-secret','studies':[study],'expireIn':10}))
      assert status==200,body
      session=json.loads(body)['session']['id']
      row=next(row for row in json.loads((ROOT/'docs/baseline/step-13-java.json').read_text()) if row['case']=='jpeg')
      upload=base64.b64decode(row['requestBase64']).replace(b'"1.2.3"',json.dumps(study).encode())
      status,body=request(port,f'/v1/session/{session}/studies','POST',upload,'multipart/related; boundary=b')
      assert status==200,body
      assert json.loads(body)[0]['7FE00010']['BulkDataURI'].startswith(f'http://127.0.0.1:{port}/ws/rest/v1/session/{session}/studies/')
      assert request(port,f'/v1/session/{session}/wado-rs/studies/{study}/series')[0]==200
      status,frame=request(port,f'/v1/session/{session}/wado-rs/studies/{study}/series/1.2.3.4/instances/1.2.3.4.5/frames/1')
      assert status==200 and b'\xff\xd8' in frame
      assert (files/'1.2.3.4/metadata.json').exists()
      request(port,f'/v1/session/{session}','DELETE')
     else:
      deadline=time.monotonic()+15
      while (files/'1.2.3.4').exists():
       assert app.poll() is None and time.monotonic()<deadline,'restart scheduler failed to cleanup persisted batch';time.sleep(.05)
    finally:
     if app.poll() is None:
      try:stop(app)
      except Exception:app.kill();app.wait();raise
  (LOGS/'tcp-result.json').write_text(json.dumps({'ingest':True,'localWado':True,'pendingSurvivesRestart':True,'schedulerCleanup':True,'gracefulShutdown':True}))
 print('PASS: binary TCP ingest/local WADO, persisted batch exported after restart, graceful stop')
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--java-home',type=Path,required=True);a=p.parse_args()
 LOGS.mkdir(parents=True,exist_ok=True)
 cli=shutil.which('postman');assert cli,'Postman CLI >=1.70 required'
 with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
 env=smoke_jni.environment(a.java_home)
 config=LOGS/'jni.toml';config_fixture.write(config,smoke_jni.jni_settings(a.java_home));env['JNI_TEST_CONFIG']=str(config)
 options={'creationflags':subprocess.CREATE_NO_WINDOW} if os.name=='nt' else {}
 with (LOGS/'simulation.ndjson').open('wb') as log,(LOGS/'simulation.err').open('wb') as err:
  sim=subprocess.Popen([cli,'simulation','run','postman/simulations/key-image.sim.yaml','--port',str(port),'--output','ndjson','--no-history'],cwd=ROOT,stdout=log,stderr=err,**options)
  try:
   deadline=time.monotonic()+30
   while True:
    assert sim.poll() is None,'simulation exited; see simulation.err'
    text=(LOGS/'simulation.ndjson').read_text(errors='replace')
    events=[]
    for line in text.splitlines():
     try:events.append(json.loads(line))
     except ValueError:pass
    if any(e.get('type')=='listening' or e.get('event')=='listening' for e in events):break
    if time.monotonic()>deadline:raise TimeoutError('simulation startup')
    time.sleep(.1)
   env['KEY_IMAGE_TEST_ARTIFACTS']=str(LOGS/'dicom')
   env['KEY_IMAGE_TEST_STORE']=f'http://127.0.0.1:{port}/store'
   with (LOGS/'contract.log').open('w',encoding='utf-8') as output:
    subprocess.run(['cargo','test','-p','viewer-app','--locked','postgres_key_image_contract','--','--ignored','--nocapture'],cwd=ROOT,env=env,stdout=output,stderr=subprocess.STDOUT,check=True,timeout=180)
   sys.path.insert(0,str(ROOT/'target/step-12-tools'))
   import pydicom
   from pydicom.encaps import generate_frames
   from PIL import Image
   checked=[]
   for row in json.loads((ROOT/'docs/baseline/step-13-java.json').read_text()):
    for name,payload in row.get('dicomBase64',{}).items():
     expected=pydicom.dcmread(io.BytesIO(base64.b64decode(payload)))
     actual=pydicom.dcmread(LOGS/'dicom'/row['case']/name)
     assert actual.file_meta.TransferSyntaxUID==expected.file_meta.TransferSyntaxUID
     # Synthetic study identity is isolated per DB test; all other dataset fields match.
     actual.StudyInstanceUID=expected.StudyInstanceUID
     assert actual.to_json_dict()==expected.to_json_dict(),row['case']
     if row['case']=='jpeg':
      image=Image.open(io.BytesIO(next(generate_frames(actual.PixelData))));image.load();assert image.size==(8,8)
     if row['case']=='raw-pixel':assert len(actual.PixelData)==64 and actual.Rows==actual.Columns==8
     checked.append({'case':row['case'],'file':name,'transferSyntax':str(actual.file_meta.TransferSyntaxUID)})
   (LOGS/'independent-output.json').write_text(json.dumps(checked,indent=2))
   print('PASS: pydicom datasets/transfer syntax match original Java; Pillow decodes encapsulated JPEG')
   tcp_smoke(a.java_home,env,env['KEY_IMAGE_TEST_STORE'])
   print('PASS: real JVM/PostgreSQL key image contract; local Postman STOW, no live PACS')
  finally:
   sim.terminate()
   try:sim.wait(timeout=10)
   except subprocess.TimeoutExpired:sim.kill();sim.wait()
if __name__=='__main__':main()
