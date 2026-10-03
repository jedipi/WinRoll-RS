const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const html = fs.readFileSync(`${__dirname}/index.html`, 'utf8');
const css = fs.readFileSync(`${__dirname}/styles.css`, 'utf8');
const ids = [...html.matchAll(/\bid="([^"]+)"/g)].map(match => match[1]);
assert.equal(new Set(ids).size, ids.length, 'HTML IDs must be unique');
for (const [, id] of html.matchAll(/href="#([^"]+)"/g)) {
  assert(ids.includes(id), `Missing anchor: ${id}`);
}

for (const [, src] of html.matchAll(/<(?:img|source|track)\b[^>]*src="([^"]+)"/g)) {
  assert(fs.existsSync(path.join(__dirname, src)), `Missing media: ${src}`);
}
for (const [, poster] of html.matchAll(/\bposter="([^"]+)"/g)) {
  assert(fs.existsSync(path.join(__dirname, poster)), `Missing poster: ${poster}`);
}
for (const [, href] of html.matchAll(/<link\b[^>]*href="([^"]+)"/g)) {
  assert(fs.existsSync(path.join(__dirname, href)), `Missing linked asset: ${href}`);
}
for (const [, refs] of html.matchAll(/aria-labelledby="([^"]+)"/g)) {
  for (const id of refs.split(' ')) assert(ids.includes(id), `Missing accessible label: ${id}`);
}
for (const [, asset] of css.matchAll(/url\("([^"]+)"\)/g)) {
  assert(fs.existsSync(path.join(__dirname, asset)), `Missing CSS asset: ${asset}`);
}
console.log('Passed: unique IDs, internal links, accessible labels, and local assets.');
