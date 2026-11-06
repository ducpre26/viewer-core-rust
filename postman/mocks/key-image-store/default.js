// Synthetic STOW responses exercise Java's HTTP-status-only acceptance rule.
const http = require('http');
let status = 202;
const requests = [];
http.createServer((req,res) => {
 const url=new URL(req.url,'http://fixture.invalid');
 if(url.pathname==='/__control'){
  if(url.searchParams.has('status'))status=Number(url.searchParams.get('status'));
  res.writeHead(200,{'Content-Type':'application/json'});res.end(JSON.stringify({requests}));return;
 }
 if(req.method==='GET' && (url.pathname.endsWith('/series')||url.pathname.endsWith('/metadata'))){res.writeHead(200,{'Content-Type':'application/dicom+json'});res.end('[]');return;}
 if(req.method!=='POST'||url.pathname!=='/studies'){res.writeHead(501);res.end('unmodeled');return;}
 const chunks=[];req.on('data',c=>chunks.push(c));req.on('end',()=>{
  const body=Buffer.concat(chunks);
  requests.push({authorization:req.headers.authorization,metadata:body.includes(Buffer.from('BulkDataURI')),dicom:body.includes(Buffer.from('DICM')),contentType:req.headers['content-type']});
  res.writeHead(status,{'Content-Type':'application/dicom+json'});
  res.end(status<300?JSON.stringify([{'00081198':{vr:'SQ',Value:[{'00081155':{vr:'UI',Value:['1.2.3.4.5']}}]}}]):'synthetic rejection');
 });
}).listen(Number(process.env.PORT),'127.0.0.1');
