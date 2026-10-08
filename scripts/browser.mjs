import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile, mkdtemp, rm } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import assert from 'node:assert/strict';
const root=new URL('../web/',import.meta.url);
const types={'/intersections.html':'text/html','/intersections.js':'text/javascript','/classification.html':'text/html','/classification.js':'text/javascript','/':'text/html','/index.html':'text/html','/app.js':'text/javascript','/style.css':'text/css','/hagane.wasm':'application/wasm','/nurbs.html':'text/html','/nurbs.js':'text/javascript','/surface.html':'text/html','/surface.js':'text/javascript','/viewer.js':'text/javascript'};
const server=createServer(async(req,res)=>{try{const path=new URL(req.url,'http://localhost').pathname;if(!types[path]){res.writeHead(404);res.end();return;}const data=await readFile(new URL(path==='/'?'index.html':path.slice(1),root));res.writeHead(200,{'Content-Type':types[path]});res.end(data);}catch{res.writeHead(500);res.end('Build the WASM module first.');}});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
let browser;
try {
 const executable=process.env.HAGANE_CHROMIUM ?? (existsSync('/usr/bin/chromium')?'/usr/bin/chromium':undefined);
 browser=await chromium.launch({executablePath:executable,headless:true,args:['--enable-unsafe-swiftshader','--use-gl=angle','--use-angle=swiftshader']});
 const page=await browser.newPage({viewport:{width:1440,height:900},deviceScaleFactor:1});const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(`http://127.0.0.1:${server.address().port}/`);await page.waitForFunction(()=>window.haganeDemo?.ready);
 const volume=await page.evaluate(()=>window.haganeDemo.mesh.volume);assert.ok(Math.abs(volume-(115200-Math.PI*196*24))<1e-8);
 const initial=await page.locator('canvas').screenshot();await page.locator('canvas').focus();await page.keyboard.press('ArrowRight');assert.notDeepEqual(await page.locator('canvas').screenshot(),initial);
 const canvas=await page.locator('canvas').boundingBox();await page.mouse.move(canvas.x+canvas.width/2,canvas.y+canvas.height/2);await page.mouse.down();await page.mouse.move(canvas.x+canvas.width/2+80,canvas.y+canvas.height/2+20,{steps:6});await page.mouse.up();const orbit=await page.locator('canvas').screenshot();assert.notDeepEqual(orbit,initial);
 await page.mouse.wheel(0,150);await page.waitForTimeout(100);assert.notDeepEqual(await page.locator('canvas').screenshot(),orbit);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(115200-Math.PI*24**2*24))<1e-8);assert.match(await page.locator('#radius-label').textContent(),/24.0/);
 const solid=await page.locator('canvas').screenshot();await page.locator('#wire').check();assert.notDeepEqual(await page.locator('canvas').screenshot(),solid);await page.locator('#wire').uncheck();
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});await page.locator('#reset').click();
 assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);
 assert.deepEqual(errors,[]);
 const presets=[{volume:115200-Math.PI*196*24,faces:7,edges:15},{volume:115200-Math.PI*196*24,faces:10,edges:24},{volume:69336,faces:12,edges:30},{volume:Math.PI*704*24,faces:4,edges:6},{volume:115200-Math.PI*196*24,faces:10,edges:24},{volume:(4800-(4-Math.PI)*196)*24,faces:10,edges:24},{volume:(4600+(4-Math.PI)*9-Math.PI*196/2)*24,faces:16,edges:42},{volume:115200-Math.PI*49*24,faces:8,edges:18},{volume:(4800-(4-Math.PI)*576)*24,faces:13,edges:31},{volume:103680,faces:15,edges:45},{volume:5376*Math.PI,faces:11,edges:26},{volume:115200-3456*Math.PI,faces:13,edges:34},{volume:115200,faces:6,edges:13},{volume:67200,faces:6,edges:12},{volume:(4096-(32*Math.SQRT2-38)**2-(32*Math.SQRT2-42)**2-2*(32*Math.SQRT2-30)**2)*24,faces:10,edges:24},{volume:107520,faces:20,edges:44},{volume:174720,faces:24,edges:52},{volume:122880,faces:26,edges:52},{volume:122880,faces:10,edges:34},{volume:174720,faces:12,edges:38},{volume:122880,faces:10,edges:24},{volume:(4600+(4-Math.PI)*9-Math.PI*196/2)*24,faces:16,edges:42},{volume:(4600+(4-Math.PI)*9-Math.PI*196/2)*24,faces:16,edges:42}];
 for(let id=0;id<presets.length;id++){
  await page.locator('#preset').selectOption(String(id));const mesh=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(mesh.volume-presets[id].volume)<1e-8);assert.equal(mesh.faces,presets[id].faces);assert.equal(mesh.edges,presets[id].edges);assert.equal(await page.locator('#radius').isDisabled(),id===2);assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);
 }
 await page.locator('#preset').selectOption('5');
 assert.match(await page.locator('#radius-title').textContent(),/Corner radius/);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(4800-(4-Math.PI)*576)*24)<1e-8);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-mixed')){await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/mixed-profile.png',import.meta.url).pathname});}
 await page.locator('#preset').selectOption('6');
 assert.match(await page.locator('#radius-title').textContent(),/Notch radius/);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(4600+(4-Math.PI)*9-Math.PI*576/2)*24)<1e-8);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-region')){await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/mixed-region.png',import.meta.url).pathname});}
 await page.locator('#preset').selectOption('7');
 assert.match(await page.locator('#radius-title').textContent(),/Cut offset/);
 const splitBefore=await page.locator('canvas').screenshot();
 const splitPositions=await page.evaluate(()=>window.haganeDemo.mesh.positions);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(115200-Math.PI*49*24))<1e-8);
 assert.notDeepEqual(await page.evaluate(()=>window.haganeDemo.mesh.positions),splitPositions);
 await page.locator('#wire').check();assert.notDeepEqual(await page.locator('canvas').screenshot(),splitBefore);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-split')){await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/face-split.png',import.meta.url).pathname});}
 await page.locator('#wire').uncheck();
 await page.locator('#preset').selectOption('8');
 assert.match(await page.locator('#radius-title').textContent(),/Arc cut offset/);
 const arcSplitPositions=await page.evaluate(()=>window.haganeDemo.mesh.positions);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.notDeepEqual(await page.evaluate(()=>window.haganeDemo.mesh.positions),arcSplitPositions);
 assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(4800-(4-Math.PI)*576)*24)<1e-8);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-arc-split')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/arc-face-split.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('9');
 assert.match(await page.locator('#radius-title').textContent(),/Hole cut offset/);
 assert.match(await page.locator('#radius-label').textContent(),/-2.0/);
 assert.deepEqual(await page.locator('.range-label span').allTextContents(),['-8 mm','8 mm']);
 const holeCutPositions=await page.evaluate(()=>window.haganeDemo.mesh.positions);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.notDeepEqual(await page.evaluate(()=>window.haganeDemo.mesh.positions),holeCutPositions);
 assert.equal(await page.evaluate(()=>window.haganeDemo.mesh.volume),103680);
 assert.match(await page.locator('#radius-label').textContent(),/8.0/);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-cut-graph')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/cut-graph.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('10');
 assert.match(await page.locator('#radius-title').textContent(),/Annular cut offset/);
 assert.deepEqual(await page.locator('.range-label span').allTextContents(),['8 mm','24 mm']);
 const repeatedArcPositions=await page.evaluate(()=>window.haganeDemo.mesh.positions);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.notDeepEqual(await page.evaluate(()=>window.haganeDemo.mesh.positions),repeatedArcPositions);
 assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-5376*Math.PI)<1e-8);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-repeated-arcs')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/repeated-arc-split.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('11');
 assert.match(await page.locator('#radius-title').textContent(),/Periodic cut offset/);
 assert.match(await page.locator('#radius-label').textContent(),/-2.0/);
 assert.deepEqual(await page.locator('.range-label span').allTextContents(),['-8 mm','8 mm']);
 const periodicPositions=await page.evaluate(()=>window.haganeDemo.mesh.positions);
 await page.locator('#radius').evaluate(e=>{e.value=16;e.dispatchEvent(new Event('input'));});
 assert.notDeepEqual(await page.evaluate(()=>window.haganeDemo.mesh.positions),periodicPositions);
 assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(115200-3456*Math.PI))<1e-8);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-periodic')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/periodic-face-split.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('12');
 assert.match(await page.locator('#radius-title').textContent(),/Boundary subdivision/);
 assert.match(await page.locator('#radius-label').textContent(),/43.8%/);
 assert.deepEqual(await page.locator('.range-label span').allTextContents(),['25%','75%']);
 const sewnPositions=await page.evaluate(()=>window.haganeDemo.mesh.positions);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.notDeepEqual(await page.evaluate(()=>window.haganeDemo.mesh.positions),sewnPositions);
 assert.equal(await page.evaluate(()=>window.haganeDemo.mesh.volume),115200);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-sewing')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/sewing.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('13');
 assert.match(await page.locator('#radius-title').textContent(),/Plane offset/);
 assert.match(await page.locator('#radius-label').textContent(),/-2.0/);
 assert.deepEqual(await page.locator('.range-label span').allTextContents(),['-8 mm','8 mm']);
 assert.equal(await page.evaluate(()=>window.haganeDemo.mesh.volume),67200);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});assert.equal(await page.evaluate(()=>window.haganeDemo.mesh.volume),19200);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-solid-split')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/solid-split.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('14');
 assert.match(await page.locator('#radius-title').textContent(),/Cutter offset/);
 assert.match(await page.locator('#radius-label').textContent(),/-2.0/);
 assert.deepEqual(await page.locator('.range-label span').allTextContents(),['-8 mm','8 mm']);
 const convexBefore=await page.evaluate(()=>window.haganeDemo.mesh);
 assert.equal(convexBefore.faces,10);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});const convexAfter=await page.evaluate(()=>window.haganeDemo.mesh);assert.equal(convexAfter.faces,9);assert.notDeepEqual(convexAfter.positions,convexBefore.positions);
 const diamond=32*Math.SQRT2;assert.ok(Math.abs(convexAfter.volume-(4096-(diamond-32)**2-2*(diamond-30)**2)*24)<1e-8);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-convex')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/convex-intersection.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('15');
 assert.match(await page.locator('#radius-title').textContent(),/Cutter offset/);
 assert.match(await page.locator('#radius-label').textContent(),/-2.0/);
 assert.deepEqual(await page.locator('.range-label span').allTextContents(),['-8 mm','8 mm']);
 const differenceBefore=await page.evaluate(()=>window.haganeDemo.mesh);assert.equal(differenceBefore.volume,107520);assert.equal(differenceBefore.faces,20);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});const differenceAfter=await page.evaluate(()=>window.haganeDemo.mesh);assert.equal(differenceAfter.volume,107520);assert.notDeepEqual(differenceAfter.positions,differenceBefore.positions);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-difference')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/convex-difference.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('16');
 assert.match(await page.locator('#radius-title').textContent(),/Cutter offset/);
 assert.deepEqual(await page.locator('.range-label span').allTextContents(),['-8 mm','8 mm']);
 const unionBefore=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(unionBefore.volume-174720)<1e-8);assert.equal(unionBefore.faces,24);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 const unionAfter=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(unionAfter.volume-181120)<1e-8);assert.notDeepEqual(unionAfter.positions,unionBefore.positions);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-union')){await page.locator('#wire').check();await page.locator('#reset').click();await page.locator('canvas').dispatchEvent('wheel',{deltaY:250});await page.screenshot({path:new URL('../docs/convex-union.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('17');
 assert.match(await page.locator('#radius-title').textContent(),/Cutter offset/);
 const contactBefore=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(contactBefore.volume-122880)<1e-8);assert.equal(contactBefore.faces,26);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 const contactAfter=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(contactAfter.volume-122880)<1e-8);assert.notDeepEqual(contactAfter.positions,contactBefore.positions);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-contact')){await page.locator('#wire').check();await page.locator('#reset').click();await page.locator('canvas').dispatchEvent('wheel',{deltaY:180});await page.screenshot({path:new URL('../docs/box-contact.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('18');
 const mergedBefore=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(mergedBefore.volume-contactBefore.volume)<1e-8);assert.equal(mergedBefore.faces,10);assert.equal(mergedBefore.edges,34);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 const mergedAfter=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(mergedAfter.volume-122880)<1e-8);assert.equal(mergedAfter.faces,10);assert.notDeepEqual(mergedAfter.positions,mergedBefore.positions);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-merge')){await page.locator('#wire').check();await page.locator('#reset').click();await page.locator('canvas').dispatchEvent('wheel',{deltaY:180});await page.screenshot({path:new URL('../docs/face-merge.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('19');
 const reframedBefore=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(reframedBefore.volume-174720)<1e-8);assert.equal(reframedBefore.faces,12);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 const reframedAfter=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(reframedAfter.volume-181120)<1e-8);assert.notDeepEqual(reframedAfter.positions,reframedBefore.positions);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-reframed')){await page.locator('#wire').check();await page.locator('#reset').click();await page.locator('canvas').dispatchEvent('wheel',{deltaY:300});await page.screenshot({path:new URL('../docs/reframed-merge.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('20');
 const simplifiedBefore=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(simplifiedBefore.volume-contactBefore.volume)<1e-8);assert.equal(simplifiedBefore.faces,10);assert.equal(simplifiedBefore.edges,24);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 const simplifiedAfter=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(simplifiedAfter.volume-122880)<1e-8);assert.equal(simplifiedAfter.edges,24);assert.notDeepEqual(simplifiedAfter.positions,simplifiedBefore.positions);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-simplify')){await page.locator('#wire').check();await page.locator('#reset').click();await page.locator('canvas').dispatchEvent('wheel',{deltaY:180});await page.screenshot({path:new URL('../docs/edge-simplify.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('21');
 assert.match(await page.locator('#radius-title').textContent(),/Notch radius/);
 const framedBefore=await page.evaluate(()=>window.haganeDemo.mesh);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 const framedAfter=await page.evaluate(()=>window.haganeDemo.mesh);
 assert.ok(Math.abs(framedAfter.volume-(4600+(4-Math.PI)*9-Math.PI*576/2)*24)<1e-8);
 assert.notDeepEqual(framedAfter.positions,framedBefore.positions);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-framed-arc')){await page.locator('#wire').uncheck();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/framed-arc-extrusion.png',import.meta.url).pathname});}
 await page.locator('#preset').selectOption('22');
 assert.match(await page.locator('#radius-title').textContent(),/Skew offset/);
 const skewBefore=await page.evaluate(()=>window.haganeDemo.mesh);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 const skewAfter=await page.evaluate(()=>window.haganeDemo.mesh);
 assert.ok(Math.abs(skewAfter.volume-skewBefore.volume)<1e-8);
 assert.notDeepEqual(skewAfter.positions,skewBefore.positions);assert.notDeepEqual(skewAfter.normals,skewBefore.normals);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-skew-arc')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/skew-arc-extrusion.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('23');assert.equal(await page.locator('#radius-title').textContent(),'Wall split position');assert.deepEqual(await page.locator('.range-label span').allTextContents(),['25%','75%']);
 const skewSplitBefore=await page.evaluate(()=>window.haganeDemo.mesh);assert.equal(skewSplitBefore.faces,22);assert.equal(skewSplitBefore.edges,58);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});const skewSplitAfter=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(skewSplitAfter.volume-skewSplitBefore.volume)<1e-8);assert.notDeepEqual(skewSplitAfter.positions,skewSplitBefore.positions);assert.equal(skewSplitAfter.faces,22);assert.equal(skewSplitAfter.edges,58);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-skew-face-split')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/skew-face-subdivision.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('24');assert.equal(await page.locator('#radius-title').textContent(),'Plane tilt');assert.deepEqual(await page.locator('.range-label span').allTextContents(),['−9.5°','9.5°']);
 const obliqueBefore=await page.evaluate(()=>window.haganeDemo.mesh);assert.equal(obliqueBefore.faces,34);assert.equal(obliqueBefore.edges,80);assert.ok(obliqueBefore.section_segments.length>16);const obliqueCanvas=await page.locator('canvas').screenshot();
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});const obliqueAfter=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(obliqueAfter.volume-obliqueBefore.volume)<1e-8);assert.notDeepEqual(obliqueAfter.positions,obliqueBefore.positions);assert.notDeepEqual(obliqueAfter.section_segments,obliqueBefore.section_segments);assert.notDeepEqual(await page.locator('canvas').screenshot(),obliqueCanvas);assert.equal(obliqueAfter.faces,34);assert.equal(obliqueAfter.edges,80);
 for(const segment of obliqueAfter.section_segments)for(const p of segment)assert.ok(Math.abs((p[0]-6)/6-0.08*(p[1]+3)+p[2])<1e-9);
 const cyanPixels=await page.evaluate(()=>{const canvas=document.getElementById('view'),gl=canvas.getContext('webgl'),pixels=new Uint8Array(canvas.width*canvas.height*4);gl.readPixels(0,0,canvas.width,canvas.height,gl.RGBA,gl.UNSIGNED_BYTE,pixels);let count=0;for(let i=0;i<pixels.length;i+=4)if(pixels[i]>=80&&pixels[i]<=140&&pixels[i+1]>=200&&pixels[i+2]>=180)count++;return count;});assert.ok(cyanPixels>100,'Actual section contours must render visibly');
 if(process.argv.includes('--capture-oblique-boundary')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/oblique-boundary.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});


 await page.locator('#preset').selectOption('1');
 const unplaced=await page.evaluate(()=>window.haganeDemo.mesh);
 await page.locator('#preset').selectOption('4');
 const placed=await page.evaluate(()=>window.haganeDemo.mesh);
 assert.notDeepEqual(placed.positions,unplaced.positions);assert.notDeepEqual(placed.normals,unplaced.normals);
 if(process.argv.includes('--capture-placement')){await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/placement.png',import.meta.url).pathname});}
 await page.locator('#preset').selectOption('0');
 if(process.argv.includes('--capture')){
  const frames=await mkdtemp(join(tmpdir(),'hagane-demo-'));
  for(let i=0;i<64;i++){if(i%16===0)await page.locator('#preset').selectOption(String(Math.floor(i/16)));await page.evaluate(a=>window.haganeDemo.setAngle(a),0.65+0.8*(i%16)/16);await page.screenshot({path:join(frames,`frame-${String(i).padStart(3,'0')}.png`)});}
  const output=new URL('../docs/demo.gif',import.meta.url).pathname;
  const result=spawnSync('ffmpeg',['-y','-framerate','12','-i',join(frames,'frame-%03d.png'),'-filter_complex','[0:v]scale=1008:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3','-loop','0',output],{encoding:'utf8'});
  if(result.status!==0)throw Error(result.stderr);console.log('Recorded actual WASM/WebGL demo to docs/demo.gif (64 frames, four exact B-rep operations).');await rm(frames,{recursive:true,force:true});
 }
 await page.setViewportSize({width:390,height:844});await page.waitForTimeout(100);assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);assert.deepEqual(errors,[]);
 await page.setViewportSize({width:1440,height:900});
 await page.goto(`http://127.0.0.1:${server.address().port}/nurbs.html`);await page.waitForFunction(()=>window.haganeNurbs?.ready);
 let curve=await page.evaluate(()=>window.haganeNurbs.data);assert.ok(Math.abs(curve.point[0]-Math.SQRT1_2)<1e-14);assert.ok(Math.abs(curve.point[1]-Math.SQRT1_2)<1e-14);
 const circleImage=await page.locator('canvas').screenshot();
 await page.locator('#weight').evaluate(e=>{e.value=2;e.dispatchEvent(new Event('input'));});
 curve=await page.evaluate(()=>window.haganeNurbs.data);assert.ok(Math.abs(curve.point[0]-5/6)<1e-14);assert.notDeepEqual(await page.locator('canvas').screenshot(),circleImage);
 await page.locator('#parameter').evaluate(e=>{e.value=0;e.dispatchEvent(new Event('input'));});curve=await page.evaluate(()=>window.haganeNurbs.data);assert.deepEqual(curve.point,[1,0,0]);assert.deepEqual(curve.tangent,[0,4,0]);
 await page.locator('#parameter').evaluate(e=>{e.value=0.5;e.dispatchEvent(new Event('input'));});await page.locator('#circle').click();
 curve=await page.evaluate(()=>window.haganeNurbs.data);assert.ok(Math.abs(curve.weight-Math.SQRT1_2)<1e-15);assert.ok(Math.abs(curve.point[0]-Math.SQRT1_2)<1e-14);
 if(process.argv.includes('--capture')){await page.screenshot({path:new URL('../docs/nurbs.png',import.meta.url).pathname});}
 await page.setViewportSize({width:390,height:844});await page.waitForTimeout(100);assert.ok(await page.locator('canvas').isVisible());assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);assert.deepEqual(errors,[]);
 await page.setViewportSize({width:1440,height:900});
 await page.goto(`http://127.0.0.1:${server.address().port}/surface.html`);await page.waitForFunction(()=>window.haganeSurface?.ready);
 let patch=await page.evaluate(()=>window.haganeSurface.data);assert.ok(Math.abs(patch.point[2]-8.75)<1e-13);assert.deepEqual(patch.normal,[0,0,1]);const initialPatch=await page.locator('canvas').screenshot();
 await page.locator('#height').evaluate(e=>{e.value=0;e.dispatchEvent(new Event('input'));});patch=await page.evaluate(()=>window.haganeSurface.data);assert.equal(patch.point[2],0);assert.notDeepEqual(await page.locator('canvas').screenshot(),initialPatch);
 await page.locator('#height').evaluate(e=>{e.value=35;e.dispatchEvent(new Event('input'));});await page.locator('#weight').evaluate(e=>{e.value=2;e.dispatchEvent(new Event('input'));});patch=await page.evaluate(()=>window.haganeSurface.data);assert.ok(Math.abs(patch.point[2]-14)<1e-13);
 await page.locator('#u').evaluate(e=>{e.value=0.3;e.dispatchEvent(new Event('input'));});await page.locator('#v').evaluate(e=>{e.value=0.7;e.dispatchEvent(new Event('input'));});patch=await page.evaluate(()=>window.haganeSurface.data);assert.ok(Math.abs(Math.hypot(...patch.normal)-1)<1e-13);assert.ok(Math.abs(patch.normal[0])+Math.abs(patch.normal[1])>0.01);
 const shaded=await page.locator('canvas').screenshot();await page.locator('#wire').check();assert.notDeepEqual(await page.locator('canvas').screenshot(),shaded);await page.locator('#wire').uncheck();await page.locator('canvas').focus();await page.keyboard.press('ArrowRight');assert.notDeepEqual(await page.locator('canvas').screenshot(),shaded);
 if(process.argv.includes('--capture')){await page.locator('#height').evaluate(e=>{e.value=60;e.dispatchEvent(new Event('input'));});await page.locator('#weight').evaluate(e=>{e.value=2;e.dispatchEvent(new Event('input'));});await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/nurbs-surface.png',import.meta.url).pathname});}
 assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);await page.setViewportSize({width:390,height:844});await page.waitForTimeout(100);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);assert.deepEqual(errors,[]);
 await page.setViewportSize({width:1440,height:900});
 await page.goto(`http://127.0.0.1:${server.address().port}/classification.html`);await page.waitForFunction(()=>window.haganeClassification?.ready);
 for(const [id,result] of [['material','inside'],['hole','outside'],['notch','outside'],['face','boundary'],['vertex','boundary'],['outside','outside']]){
  await page.locator('#probe').selectOption(id);assert.equal(await page.locator('#location').textContent(),result);assert.equal(await page.evaluate(()=>window.haganeClassification.data.location),result);
 }
 await page.locator('#probe').selectOption('material');
 await page.locator('#x').evaluate(e=>{e.value=-26;e.dispatchEvent(new Event('input'));});assert.equal(await page.locator('#location').textContent(),'outside');assert.equal(await page.locator('#probe').inputValue(),'custom');
 await page.locator('#x').evaluate(e=>{e.value=-20;e.dispatchEvent(new Event('input'));});assert.equal(await page.locator('#location').textContent(),'boundary');
 await page.locator('#probe').selectOption('face');await page.locator('#wire').check();await page.locator('#reset').click();
 if(process.argv.includes('--capture-classification'))await page.screenshot({path:new URL('../docs/classification.png',import.meta.url).pathname});
 for(const [model,expected] of [['1',{material:'inside',hole:'outside',notch:'boundary',face:'boundary',vertex:'boundary',outside:'outside'}],['2',{material:'inside',hole:'inside',notch:'boundary',face:'boundary',vertex:'boundary',outside:'outside'}],['3',{material:'inside',hole:'outside',notch:'boundary',face:'boundary',vertex:'boundary',outside:'outside'}],['4',{material:'inside',hole:'outside',notch:'boundary',face:'boundary',vertex:'boundary',outside:'outside'}],['5',{material:'inside',hole:'outside',notch:'boundary',face:'boundary',vertex:'boundary',outside:'outside'}],['6',{material:'inside',hole:'outside',notch:'boundary',face:'boundary',vertex:'boundary',outside:'outside'}],['7',{material:'inside',hole:'outside',notch:'boundary',face:'boundary',vertex:'boundary',outside:'outside'}]]){
  await page.locator('#model').selectOption(model);assert.equal(await page.evaluate(()=>window.haganeClassification.model),Number(model));
  for(const [probe,result] of Object.entries(expected)){await page.locator('#probe').selectOption(probe);assert.equal(await page.locator('#location').textContent(),result);}
 }
 await page.locator('#model').selectOption('1');await page.locator('#probe').selectOption('notch');
 if(process.argv.includes('--capture-curved-classification'))await page.screenshot({path:new URL('../docs/curved-classification.png',import.meta.url).pathname});
 await page.locator('#x').evaluate(e=>{e.value=0;e.dispatchEvent(new Event('input'));});assert.equal(await page.locator('#location').textContent(),'outside');
 if(process.argv.includes('--capture-arc-classification')){await page.locator('#model').selectOption('5');await page.locator('#probe').selectOption('notch');await page.screenshot({path:new URL('../docs/arc-classification.png',import.meta.url).pathname});}
 if(process.argv.includes('--capture-skew-classification')){await page.locator('#wire').uncheck();await page.locator('#model').selectOption('6');await page.locator('#probe').selectOption('notch');await page.screenshot({path:new URL('../docs/skew-classification.png',import.meta.url).pathname});}
 if(process.argv.includes('--capture-harmonic-classification')){
  await page.locator('#model').selectOption('7');await page.locator('#wire').check();await page.locator('#probe').selectOption('notch');await page.locator('#reset').click();
  await page.screenshot({path:new URL('../docs/harmonic-classification.png',import.meta.url).pathname});await page.locator('#wire').uncheck();
 }
 await page.locator('#model').selectOption('0');await page.locator('#probe').selectOption('face');assert.equal(await page.locator('#location').textContent(),'boundary');
 const classifierView=await page.locator('canvas').screenshot();await page.locator('canvas').focus();await page.keyboard.press('ArrowRight');assert.notDeepEqual(await page.locator('canvas').screenshot(),classifierView);
 assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);await page.setViewportSize({width:390,height:844});await page.waitForTimeout(100);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);assert.deepEqual(errors,[]);
 console.log('Browser: planar and full-cylinder solid point classification, eight models including harmonic circular wall bands, material/hole/notch/boundary probes, sliders, orbit and responsive layout passed.');
 console.log('Browser: NURBS surface height/weight/UV, analytic point/normal, open-patch shading, wireframe, orbit and responsive layout passed.');
 console.log('Browser: NURBS weight/parameter controls, exact-circle reset, native-derived coordinates, canvas changes and responsive layout passed.');
 await page.setViewportSize({width:1440,height:900});await page.goto(`http://127.0.0.1:${server.address().port}/intersections.html`);await page.waitForFunction(()=>window.haganeIntersections?.ready);
 for(const [probe,kind,hits] of [['crossing','crossing','2'],['tangent','tangent','1'],['empty','empty','0'],['generator','generator overlap','0'],['reverse','generator overlap','0']]){
  await page.locator('#probe').selectOption(probe);assert.equal(await page.locator('#kind').textContent(),kind);assert.equal(await page.locator('#hits').textContent(),hits);assert.ok(await page.evaluate(()=>window.haganeIntersections.data));
 }
 await page.locator('#probe').selectOption('crossing');const intersectionsBefore=await page.locator('canvas').screenshot();await page.locator('#offset').evaluate(e=>{e.value=0;e.dispatchEvent(new Event('input'));});assert.equal(await page.locator('#kind').textContent(),'crossing');assert.equal(await page.locator('#probe').inputValue(),'custom');assert.notDeepEqual(await page.locator('canvas').screenshot(),intersectionsBefore);
 await page.locator('#probe').selectOption('tangent');await page.locator('#offset').evaluate(e=>{e.step='any';e.value=24+1e-9;e.dispatchEvent(new Event('input'));});assert.equal(await page.locator('#kind').textContent(),'unresolved');assert.equal(await page.evaluate(()=>window.haganeIntersections.data),null);
 await page.locator('#probe').selectOption('crossing');assert.equal(await page.locator('#kind').textContent(),'crossing');await page.locator('#wire').check();await page.locator('#reset').click();
 if(process.argv.includes('--capture-extrusion-intersections'))await page.screenshot({path:new URL('../docs/extrusion-intersections.png',import.meta.url).pathname});
 const hitView=await page.locator('canvas').screenshot();await page.locator('canvas').focus();await page.keyboard.press('ArrowRight');assert.notDeepEqual(await page.locator('canvas').screenshot(),hitView);
 await page.locator('#scope').selectOption('0');await page.locator('#probe').selectOption('crossing');assert.equal(await page.locator('#hits').textContent(),'2');assert.equal(await page.evaluate(()=>window.haganeIntersections.data.face),2);
 const selectedMesh=await page.evaluate(()=>window.haganeIntersections.data.display_mesh);assert.ok(selectedMesh.positions.length<await page.evaluate(()=>window.haganeIntersections.data.mesh.positions.length));
 await page.locator('#offset').evaluate(e=>{e.value=-14;e.dispatchEvent(new Event('input'));});assert.equal(await page.locator('#kind').textContent(),'empty');
 await page.locator('#scope').selectOption('1');assert.equal(await page.locator('#kind').textContent(),'crossing');assert.equal(await page.locator('#hits').textContent(),'2');
 await page.locator('#probe').selectOption('generator');assert.equal(await page.locator('#kind').textContent(),'generator overlap');assert.equal(await page.evaluate(()=>window.haganeIntersections.data.intersection.endpoints[0].boundaries.length),2);
 await page.locator('#scope').selectOption('0');await page.locator('#offset').evaluate(e=>{e.value=1e-9;e.dispatchEvent(new Event('input'));});assert.equal(await page.locator('#kind').textContent(),'unresolved');
 await page.locator('#offset').evaluate(e=>{e.value=0;e.dispatchEvent(new Event('input'));});assert.equal(await page.locator('#kind').textContent(),'generator overlap');
 await page.locator('#mode').selectOption('0');assert.equal(await page.locator('#kind').textContent(),'crossing');assert.equal(await page.evaluate(()=>window.haganeIntersections.data.intersection.hits.every(p=>p.boundaries.length===1 && p.boundaries[0].edge_parameter===0.5)),true);
 await page.locator('#probe').selectOption('crossing');await page.locator('#wire').check();await page.locator('#reset').click();
 if(process.argv.includes('--capture-circular-face-intersections'))await page.screenshot({path:new URL('../docs/circular-face-intersections.png',import.meta.url).pathname});
 assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);assert.deepEqual(errors,[]);
 console.log('Browser: circular face trim selection, original edge parameters, exact boundary overlap, ambiguity recovery and selected face meshes passed.');
 console.log('Browser: skew surface secants, tangent, empty, forward/reverse generator, ambiguity recovery, slider, marker lines, orbit and responsive layout passed.');
 await page.setViewportSize({width:1440,height:900});
 for(const scope of ['band0','band1']){
  await page.locator('#scope').selectOption(scope);await page.locator('#probe').selectOption('crossing');
  assert.equal(await page.locator('#hits').textContent(),'1');
  assert.ok(await page.evaluate(()=>window.haganeIntersections.data.display_mesh.positions.length>0));
  await page.locator('#probe').selectOption('generator');assert.equal(await page.locator('#kind').textContent(),'generator overlap');
 }
 await page.locator('#probe').selectOption('ellipse');
 const lower=await page.evaluate(()=>window.haganeIntersections.data.intersection.hits[0]);
 assert.equal(await page.locator('#hits').textContent(),'1');assert.equal(lower.boundaries.length,1);
 await page.locator('#scope').selectOption('band1');
 const upper=await page.evaluate(()=>window.haganeIntersections.data.intersection.hits[0]);
 assert.equal(upper.boundaries[0].edge,lower.boundaries[0].edge);
 assert.ok(Math.abs(upper.boundaries[0].edge_parameter-Math.PI/2)<1e-10);
 await page.locator('#offset').evaluate(e=>{e.step='any';e.value=1e-9;e.dispatchEvent(new Event('input'));});
 assert.equal(await page.locator('#kind').textContent(),'unresolved');
 await page.locator('#probe').selectOption('ellipse');await page.locator('#wire').uncheck();await page.locator('#reset').click();
 if(process.argv.includes('--capture-harmonic-face-intersections'))await page.screenshot({path:new URL('../docs/harmonic-face-intersections.png',import.meta.url).pathname});
 assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);
 assert.deepEqual(errors,[]);
 console.log('Browser: harmonic wall bands, clipped roots/generators, shared ellipse provenance, ambiguity and recovery passed.');
 await page.locator('#scope').selectOption('ellipsePlane');assert.equal(await page.locator('#hits').textContent(),'2');
 const ellipseData=await page.evaluate(()=>window.haganeIntersections.data);
 assert.equal(ellipseData.mesh.faces,4);assert.equal(ellipseData.intersection.intervals.length,1);
 assert.ok(ellipseData.intersection.hits.every(p=>p.boundaries.length===1));
 assert.ok(Math.abs(ellipseData.mesh.volume-Math.PI*24**2*12)<1e-8);
 assert.ok(await page.locator('#mode option[value="1"]').isDisabled());
 await page.locator('#probe').selectOption('tangent');assert.equal(await page.locator('#kind').textContent(),'unresolved');
 await page.locator('#probe').selectOption('empty');assert.equal(await page.locator('#kind').textContent(),'empty');
 await page.locator('#probe').selectOption('crossing');assert.equal(await page.locator('#hits').textContent(),'2');
 await page.locator('#offset').evaluate(e=>{e.value=0;e.dispatchEvent(new Event('input'));});assert.equal(await page.locator('#kind').textContent(),'unresolved');
 await page.locator('#probe').selectOption('crossing');await page.locator('#wire').uncheck();await page.locator('#reset').click();
 if(process.argv.includes('--capture-ellipse-planar'))await page.screenshot({path:new URL('../docs/ellipse-planar.png',import.meta.url).pathname});
 const ellipseView=await page.locator('canvas').screenshot();await page.locator('canvas').focus();await page.keyboard.press('ArrowRight');assert.notDeepEqual(await page.locator('canvas').screenshot(),ellipseView);
 assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);assert.deepEqual(errors,[]);
 console.log('Browser: planar ellipse cap, original boundary parameters, tangent/vertex rejection, empty clip, recovery and orbit passed.');
 console.log('Browser: 25 B-rep presets including skew circular and oblique ellipse subdivision, WASM generation, radius, keyboard/drag orbit, wheel zoom, wireframe, reset, responsive rendering passed.');
} finally {await browser?.close();server.close();}
