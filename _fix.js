const https = require('fs');
const fs = require('fs');
const path = require('path');

const TOKEN = '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';
const REPO = 'shushuzn/ai_research_os';
const SHA = 'c2ff2ad357367c7b759bd16ab54086a2e631bbf0';
const PARENT = 'd99c4eee40a37b0f699b04555ad3e3a926846e75';

function rawGet(pathname) {
  return new Promise((resolve, reject) => {
    const opts = {
      hostname: 'api.github.com',
      path: pathname,
      method: 'GET',
      headers: {
        'Authorization': 'token ' + TOKEN,
        'User-Agent': 'node-script',
        'Accept': 'application/vnd.github.v3'
      }
    };
    const req = https.request(opts, res => {
      const chunks = [];
      res.on('data', c => chunks.push(c));
      res.on('end', () => resolve(Buffer.concat(chunks)));
    });
    req.on('error', reject);
    req.end();
  });
}

function writeObject(sha, buf) {
  const subdir = path.join('.git', 'objects', sha.slice(0, 2));
  const objFile = path.join(subdir, sha.slice(2));
  fs.mkdirSync(subdir, { recursive: true });
  const zlib = require('zlib');
  const compressed = zlib.deflateSync(buf);
  fs.writeFileSync(objFile, compressed);
  console.log('Wrote', objFile, 'uncompressed size:', buf.length);
}

async function main() {
  // Fetch raw commit bytes
  console.log('Fetching raw commit...');
  const commitBuf = await rawGet('/repos/' + REPO + '/git/commits/' + SHA);
  console.log('Raw commit size:', commitBuf.length, 'bytes');

  // Write commit object
  writeObject(SHA, commitBuf);

  // Fetch parent commit bytes
  console.log('Fetching parent commit...');
  const parentBuf = await rawGet('/repos/' + REPO + '/git/commits/' + PARENT);
  console.log('Raw parent size:', parentBuf.length, 'bytes');
  writeObject(PARENT, parentBuf);

  // Now fetch the tree too
  // First decode commit to get tree SHA
  const zlib = require('zlib');
  const decompressed = zlib.inflateSync(fs.readFileSync(path.join('.git', 'objects', SHA.slice(0, 2), SHA.slice(2))));
  const content = decompressed.toString();
  const treeMatch = content.match(/^tree ([a-f0-9]{40})$/m);
  if (treeMatch) {
    console.log('Tree SHA:', treeMatch[1]);
    // Fetch tree - but we need raw bytes for it too
    // Actually tree objects should be fine since they exist in pack files or as loose objects
    // Let's just verify git works now
  }

  // Try git status
  const { execSync } = require('child_process');
  try {
    const log = execSync('git log --oneline -3', {encoding: 'utf8', timeout: 5000});
    console.log('\ngit log OK:\n', log);
    const st = execSync('git status', {encoding: 'utf8', timeout: 5000});
    console.log('git status:\n', st);
  } catch(e) {
    console.log('git error:', e.message);
  }
}

main().catch(console.error);
