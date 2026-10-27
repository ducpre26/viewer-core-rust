// Synthetic transport fixtures, derived from the recorded Java WADO contract.
// This mock is local only. It contains no real store credentials or patient data.
const http = require('http');
http.createServer((req, res) => {
  const url = new URL(req.url, 'http://fixture.invalid');
  if (!url.pathname.startsWith('/dicom/wado-rs/studies/') && url.pathname !== '/dicom/wado') {
    res.writeHead(501); res.end(); return;
  }
  if (req.headers.authorization !== 'Basic fixture-secret') {
    res.writeHead(401); res.end('fixture auth required'); return;
  }
  const status = Number(url.searchParams.get('fixtureStatus') || 200);
  res.writeHead(status, {
    'Content-Type': 'multipart/related; type="application/dicom"; boundary=fixture',
    'Set-Cookie': 'fixture=must-not-forward',
    'X-Upstream-Secret': 'must-not-forward',
  });
  res.end(Buffer.from([0, 1, 2, 13, 10, 255, 42]));
}).listen(process.env.PORT);
