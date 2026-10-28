// Local synthetic network faults, based on the Step 12 download contract.
const http = require('http');
const fs = require('fs');
const path = require('path');
const root = path.resolve(__dirname, '../../..');
const fixture = JSON.parse(fs.readFileSync(path.join(root, 'target/step-12-error-fixtures/manifest.json')));
const small = Buffer.from(fixture.dicomBase64, 'base64');
const jpeg = Buffer.from(fixture.jpegBase64, 'base64');
const state = { mode: 'ok', requests: [], pending: [], firstChunk: false };
function metadata(mode) {
  const count = mode === 'video' ? fixture.frames : 2;
  const entry = (sop, index) => ({'00080018':{vr:'UI',Value:[sop]},'00200013':{vr:'IS',Value:[index]},'00280008':{vr:'IS',Value:[count]}});
  return JSON.stringify(mode === 'second-instance-fails' ? [entry('1.2.3.4.5',1),entry('1.2.3.4.6',2)] : [entry('1.2.3.4.5',1)]);
}
function send(res, status, media, data) {
  res.writeHead(status, {'Content-Type':media,'Content-Length':data.length});res.end(data);
}
const server = http.createServer((req, res) => {
  try {
    const url=new URL(req.url,'http://fixture.invalid');
    if (url.pathname==='/__control') {
      if(url.searchParams.has('mode')) {state.mode=url.searchParams.get('mode');state.requests=[];state.firstChunk=false;}
      if(url.searchParams.has('release')) {for(const p of state.pending.splice(0)) if(!p.res.destroyed)p.res.end(p.rest);}
      const payload=Buffer.from(JSON.stringify({mode:state.mode,firstChunk:state.firstChunk,requests:state.requests,directPort:server.address().port}));
      send(res,200,'application/json',payload);return;
    }
    if(req.headers.authorization!=='Basic fixture-secret') {send(res,401,'application/json',Buffer.from('{"error":"fixture auth"}'));return;}
    const source=url.pathname==='/dicom/wado';
    const rendered=url.pathname.endsWith('/rendered');
    const meta=url.pathname.endsWith('/metadata');
    const listing=url.pathname.endsWith('/series');
    if(!source && !rendered && !meta && !listing) {send(res,501,'text/plain',Buffer.from('unmodeled'));return;}
    const mode=state.mode;
    state.requests.push({path:url.pathname,query:url.search,mode,at:Date.now()});
    if(state.requests.length>2000) state.requests.shift();
    if(meta || listing) {
      if(mode==='metadata-503') {send(res,503,'application/json',Buffer.from('{"error":"STORE_PRIVATE_MARKER"}'));return;}
      const value=listing?'[{"0020000E":{"vr":"UI","Value":["1.2.3.4"]}}]':metadata(mode);
      send(res,200,'application/dicom+json',Buffer.from(value));return;
    }
    if(mode.startsWith('status-')) {send(res,Number(mode.slice(7)),'application/json',Buffer.from('{"error":"STORE_PRIVATE_MARKER"}'));return;}
    if(mode==='html-502') {send(res,502,'text/html',Buffer.from('<html>STORE_PRIVATE_MARKER</html>'));return;}
    if(mode==='second-instance-fails' && url.searchParams.get('objectUID')==='1.2.3.4.6') {send(res,503,'application/json',Buffer.from('{}'));return;}
    if(mode==='second-frame-fails' && /\/frames\/2\/rendered$/.test(url.pathname)) {send(res,503,'application/json',Buffer.from('{}'));return;}
    const media=rendered?'image/jpeg':'application/dicom';
    const body=rendered?jpeg:small;
    if(mode==='headers-hang') {const timer=setTimeout(()=>send(res,200,media,body),5000);res.on('close',()=>clearTimeout(timer));return;}
    if(mode==='body-truncated') {
      res.writeHead(200,{'Content-Type':media,'Content-Length':body.length+100});res.flushHeaders();res.write(body.subarray(0,32));setTimeout(()=>res.destroy(),40);return;
    }
    if(mode==='body-hang' || mode==='gated') {
      const half=Math.floor(body.length/2);
      res.writeHead(200,{'Content-Type':media,'Content-Length':body.length});res.flushHeaders();res.write(body.subarray(0,half));state.firstChunk=true;
      if(mode==='gated') state.pending.push({res,rest:body.subarray(half)});
      else {const timer=setTimeout(()=>res.end(body.subarray(half)),5000);res.on('close',()=>clearTimeout(timer));}
      return;
    }
    if(mode==='video' || mode==='large-file') {
      const size=fs.statSync(fixture.videoPath).size;
      res.writeHead(200,{'Content-Type':'application/dicom','Content-Length':size});
      const stream=fs.createReadStream(fixture.videoPath);stream.on('error',()=>res.destroy());res.on('close',()=>stream.destroy());stream.pipe(res);return;
    }
    send(res,200,media,body);
  } catch(error) {if(!res.headersSent)send(res,500,'text/plain',Buffer.from('fixture internal error'));else res.destroy();}
});
server.listen(Number(process.env.PORT),'127.0.0.1');
